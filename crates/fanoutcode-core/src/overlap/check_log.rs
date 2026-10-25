//! The lines the claim-check hook appends to its task's event log: `ClaimDenied`, `ClaimNoted` and `CommandRan`.

use std::io::Write;
use std::path::Path;

use chrono::Utc;

use super::claims::Claim;
use super::grants::CommandNeed;

/// Appends `<event><TAB><rfc3339><TAB><payload>` to the task's event log in one write.
fn append_event(log: &Path, event: &str, payload: &serde_json::Value) {
    let line = format!("{event}\t{}\t{payload}\n", Utc::now().format("%Y-%m-%dT%H:%M:%SZ"));
    let written = log
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::OpenOptions::new().create(true).append(true).open(log))
        .and_then(|mut f| f.write_all(line.as_bytes()));
    if let Err(e) = written {
        eprintln!("fanoutcode claim-check: cannot log to {}: {e}", log.display());
    }
}

/// `{repo, path, command, cwd, other_task}` as `event`: `ClaimDenied` for a stop, `ClaimNoted` for an overlap
/// allowed with "Ask before editing" off.
pub(super) fn log_claim(
    log: &Path,
    event: &str,
    repo: &Path,
    path: Option<&str>,
    command: Option<&str>,
    other: Option<&Claim>,
    cwd: &Path,
) {
    let payload = serde_json::json!({
        "repo": repo,
        "path": path,
        "command": command,
        "cwd": cwd,
        "other_task": other.and_then(|c| c.task_id.as_ref()),
    });
    append_event(log, event, &payload);
}

/// One `CommandRan` line per grant just spent (§5.6): the app tells the other task only when the line matches a
/// grant it wrote itself.
pub(super) fn log_command_ran(log: &Path, command: &str, needed: &[CommandNeed]) {
    for n in needed {
        let payload = serde_json::json!({
            "repo": n.repo,
            "command": command,
            "kind": n.kind,
            "paths": n.paths,
            "other_task": n.other,
        });
        append_event(log, "CommandRan", &payload);
    }
}
