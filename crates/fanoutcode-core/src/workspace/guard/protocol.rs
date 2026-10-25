//! The hook side of the guard: Claude Code's `PreToolUse` stdin/stdout protocol, the per-task roots file, and the
//! `HookSpec` that runs `fanoutcode guard <roots-file>` before every Bash call.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::rules::clean;
use super::{evaluate, GuardDecision, GuardInput, GuardRule};
use crate::domain::TaskId;
use crate::hooks::{sh_single_quote, HookSpec};
use crate::memory::store::write_atomic;
use crate::Result;

/// `<app_data>/guards/<task-id>.guard.json`: the protected linked roots, and the app's own folders the hooks trust
/// (`app_files`, Plan 11: grants, claims, notes, events), which no shell command of the agent may write either.
#[derive(Debug, Serialize, Deserialize)]
struct RootsFile {
    protected: Vec<PathBuf>,
    #[serde(default)]
    app_files: Vec<PathBuf>,
    home_dir: PathBuf,
}

/// Why a write to one of the app's own folders is refused.
pub fn app_files_reason(dir: &Path, rule: &GuardRule) -> String {
    format!(
        "Fanout Code: {} holds Fanout Code's own records (claims, grants, notes and events between tasks); this shell \
         command looks like it writes there ({rule}). Never change these files; ask the developer instead.",
        dir.display()
    )
}

#[derive(Deserialize)]
struct HookInput {
    tool_name: Option<String>,
    tool_input: Option<serde_json::Value>,
    cwd: Option<PathBuf>,
}

/// What the guard enforces when its roots file is missing, unreadable or malformed: the session was told to read
/// it, so it was removed or emptied, perhaps by the agent itself. The whole app data folder (`<app_data>` of
/// `<app_data>/guards/<id>.guard.json`) is then guarded, so the agent cannot finish switching the guard off; the
/// protected linked roots are not known and fail open, the `Edit` deny rules still holding them.
fn fallback_roots(roots_file: &Path) -> Option<RootsFile> {
    let app_data = roots_file.parent()?.parent()?;
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    let mut app_files = vec![app_data.to_owned()];
    if let Ok(canonical) = std::fs::canonicalize(app_data) {
        if canonical != app_data {
            app_files.push(canonical);
        }
    }
    Some(RootsFile {
        protected: Vec::new(),
        app_files,
        home_dir: home,
    })
}

/// Reads one hook payload from `stdin` and prints a deny decision for a Bash command that writes under a protected
/// root or one of the app's own folders. Always returns 0. Malformed input is reported on stderr and allowed (fail
/// open; the `Edit` deny rules remain the hard guarantee); a roots file that cannot be read fails closed for the app
/// data folder only (`fallback_roots`).
pub fn run_guard(stdin: &mut dyn Read, stdout: &mut dyn Write, roots_file: &Path) -> i32 {
    let read: Option<RootsFile> = std::fs::read(roots_file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let roots = match read.or_else(|| {
        eprintln!(
            "fanoutcode guard: cannot read {}; guarding the app data folder only",
            roots_file.display()
        );
        fallback_roots(roots_file)
    }) {
        Some(r) => r,
        None => return 0,
    };
    let mut text = String::new();
    let input: HookInput = match stdin
        .read_to_string(&mut text)
        .ok()
        .and_then(|_| serde_json::from_str(&text).ok())
    {
        Some(i) => i,
        None => {
            eprintln!("fanoutcode guard: malformed hook input; allowing");
            return 0;
        }
    };
    if input.tool_name.as_deref() != Some("Bash") {
        return 0;
    }
    let Some(command) = input
        .tool_input
        .as_ref()
        .and_then(|t| t.get("command"))
        .and_then(|c| c.as_str())
    else {
        return 0;
    };
    let cwd = input.cwd.unwrap_or_else(|| PathBuf::from("/"));
    let watched: Vec<PathBuf> = roots.protected.iter().chain(&roots.app_files).cloned().collect();
    let decision = evaluate(
        &GuardInput {
            command: command.to_owned(),
            cwd,
        },
        &watched,
        &roots.home_dir,
    );
    if let GuardDecision::Deny { root, rule, reason } = decision {
        let reason = if roots.app_files.iter().any(|a| clean(a) == root) {
            app_files_reason(&root, &rule)
        } else {
            reason
        };
        let out = serde_json::json!({"hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }});
        let _ = writeln!(stdout, "{out}");
    }
    0
}

/// Writes the task's roots file (the task id re-validated, since it becomes a file name).
pub fn write_roots_file(
    guards_dir: &Path,
    task_id: &TaskId,
    protected: &[PathBuf],
    app_files: &[PathBuf],
    home_dir: &Path,
) -> Result<PathBuf> {
    let id = TaskId::parse(task_id.as_str())?;
    let path = guards_dir.join(format!("{id}.guard.json"));
    let body = RootsFile {
        protected: protected.to_vec(),
        app_files: app_files.to_vec(),
        home_dir: home_dir.to_owned(),
    };
    write_atomic(&path, &serde_json::to_vec_pretty(&body)?)?;
    Ok(path)
}

/// `'<exe>' guard '<roots_file>'` on `PreToolUse` for `Bash`, five-second timeout.
pub fn guard_hook_spec(exe: &Path, roots_file: &Path) -> HookSpec {
    HookSpec {
        event: "PreToolUse".into(),
        matcher: Some("Bash".into()),
        command: format!(
            "{} guard {}",
            sh_single_quote(&exe.display().to_string()),
            sh_single_quote(&roots_file.display().to_string())
        ),
        timeout_s: Some(5),
    }
}

/// `None` without protected roots or app folders; otherwise writes `<app_data>/guards/<task-id>.guard.json` and returns the hook to
/// add to the task's generated settings. Called at every launch.
pub fn prepare_guard(
    app_data: &Path,
    exe: &Path,
    task_id: &TaskId,
    protected: &[PathBuf],
    app_files: &[PathBuf],
    home_dir: &Path,
) -> Result<Option<HookSpec>> {
    if protected.is_empty() && app_files.is_empty() {
        return Ok(None);
    }
    let file = write_roots_file(&app_data.join("guards"), task_id, protected, app_files, home_dir)?;
    Ok(Some(guard_hook_spec(exe, &file)))
}
