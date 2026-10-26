//! Which agent CLIs this Mac can run (first run): each found on the login shell's `PATH`, the same one sessions
//! use, with the version it reports. Signing in is not checked here; a session shows Claude Code's own sign-in screen
//! on its card (`sessions::agent_prompt`).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::AgentKind;

/// How long `<cli> --version` may take before the CLI counts as found but silent.
pub const VERSION_TIMEOUT: Duration = Duration::from_secs(5);

/// One agent CLI: where it is, and its version line, when found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct AgentCli {
    pub kind: AgentKind,
    pub program: String,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

/// The program each checked agent runs as.
pub const AGENTS: [(AgentKind, &str); 2] = [(AgentKind::ClaudeCode, "claude"), (AgentKind::Codex, "codex")];

/// The first executable file named `program` in the `:`-separated `path_var`.
pub fn find_on(path_var: &str, program: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    path_var
        .split(':')
        .filter(|d| !d.is_empty())
        .map(|d| Path::new(d).join(program))
        .find(|p| std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}

/// The first non-empty line of `<path> --version`, or `None` when it fails or takes longer than `VERSION_TIMEOUT`.
pub async fn version_of(path: &Path) -> Option<String> {
    let run = tokio::process::Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(VERSION_TIMEOUT, run).await.ok()?.ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_owned)
}

/// Every agent in `AGENTS`, looked up on `path_var`.
pub async fn check(path_var: &str) -> Vec<AgentCli> {
    let mut found = Vec::new();
    for (kind, program) in AGENTS {
        let path = find_on(path_var, program);
        let version = match &path {
            Some(p) => version_of(p).await,
            None => None,
        };
        found.push(AgentCli {
            kind,
            program: program.to_owned(),
            path,
            version,
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn script(dir: &Path, name: &str, body: &str, mode: u32) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
        p
    }

    #[test]
    fn only_an_executable_file_on_the_path_counts() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        script(a.path(), "claude", "", 0o644); // not executable
        let good = script(b.path(), "claude", "", 0o755);
        let path_var = format!("{}::{}", a.path().display(), b.path().display());
        assert_eq!(find_on(&path_var, "claude"), Some(good));
        assert_eq!(find_on(&path_var, "codex"), None);
    }

    #[tokio::test]
    async fn check_reports_each_agent_with_its_version_line() {
        let dir = tempfile::tempdir().unwrap();
        script(dir.path(), "claude", "echo; echo '2.1.310 (Claude Code)'", 0o755);
        let found = check(&dir.path().display().to_string()).await;
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].kind, AgentKind::ClaudeCode);
        assert_eq!(found[0].version.as_deref(), Some("2.1.310 (Claude Code)"));
        assert_eq!((found[1].kind, found[1].path.as_ref()), (AgentKind::Codex, None));
    }

    #[tokio::test]
    async fn a_failing_version_command_is_found_but_has_no_version() {
        let dir = tempfile::tempdir().unwrap();
        let p = script(dir.path(), "codex", "exit 3", 0o755);
        assert_eq!(version_of(&p).await, None);
    }
}
