//! What a session runs: the per-task Claude Code settings file the app generates, and the launch spec for each
//! agent kind. The app never writes into a project's `.claude/` folder (§7.3); the settings file lives under
//! `<app_data>/claude-settings/` and carries only hooks, the status line, protected-root deny rules and, when
//! needed, the background-isolation key (§14).

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::domain::TaskId;
use crate::hooks::HookSpec;
use crate::ids::valid_id;
use crate::Error;

/// A program, its arguments, its working directory and the extra environment it gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

/// The generated settings file. Plan 05 fills `hooks` and `status_line`, Plan 10 `permission_deny`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeneratedSettings {
    pub hooks: Vec<HookSpec>,
    pub status_line: Option<String>,
    pub permission_deny: Vec<String>,
    pub bg_isolation_none: bool,
}

impl GeneratedSettings {
    /// Claude Code's settings shape. Hooks of one event and matcher share one matcher entry, in input order; a
    /// top-level key is left out when its content is empty or false.
    pub fn to_json(&self) -> Value {
        let mut root = Map::new();
        if !self.hooks.is_empty() {
            root.insert("hooks".into(), Value::Object(hooks_json(&self.hooks)));
        }
        if let Some(cmd) = &self.status_line {
            root.insert("statusLine".into(), json!({ "type": "command", "command": cmd }));
        }
        if !self.permission_deny.is_empty() {
            root.insert("permissions".into(), json!({ "deny": self.permission_deny }));
        }
        if self.bg_isolation_none {
            root.insert("worktree".into(), json!({ "bgIsolation": "none" }));
        }
        Value::Object(root)
    }

    /// Pretty JSON, mode 0600, written to a sibling temp file and renamed into place; creates the parent.
    pub fn write(&self, path: &Path) -> Result<(), Error> {
        let parent = path
            .parent()
            .ok_or_else(|| Error::Invalid(format!("No folder for {}", path.display())))?;
        std::fs::create_dir_all(parent)?;
        let mut text = serde_json::to_string_pretty(&self.to_json()).map_err(|e| Error::Invalid(e.to_string()))?;
        text.push('\n');
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tmp = parent.join(format!(".{name}.tmp"));
        write_private(&tmp, text.as_bytes())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

fn hooks_json(hooks: &[HookSpec]) -> Map<String, Value> {
    // (event, matcher) → the hook objects, keeping first-appearance order of each pair
    let mut groups: Vec<(&str, Option<&str>, Vec<Value>)> = Vec::new();
    for h in hooks {
        let mut hook = Map::new();
        hook.insert("type".into(), "command".into());
        hook.insert("command".into(), h.command.clone().into());
        if let Some(t) = h.timeout_s {
            hook.insert("timeout".into(), t.into());
        }
        let key = (h.event.as_str(), h.matcher.as_deref());
        match groups.iter_mut().find(|(e, m, _)| (*e, *m) == key) {
            Some((_, _, list)) => list.push(Value::Object(hook)),
            None => groups.push((key.0, key.1, vec![Value::Object(hook)])),
        }
    }
    let mut out: Map<String, Value> = Map::new();
    for (event, matcher, list) in groups {
        let mut entry = Map::new();
        if let Some(m) = matcher {
            entry.insert("matcher".into(), m.into());
        }
        entry.insert("hooks".into(), Value::Array(list));
        match out.get_mut(event) {
            Some(Value::Array(entries)) => entries.push(Value::Object(entry)),
            _ => {
                out.insert(event.to_owned(), Value::Array(vec![Value::Object(entry)]));
            }
        }
    }
    out
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    Ok(())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    std::fs::write(path, bytes)?;
    Ok(())
}

/// `<app_data>/claude-settings/<task-id>.json`; `Error::Invalid` unless the id is a valid id.
pub fn settings_path(app_data: &Path, task_id: &TaskId) -> Result<PathBuf, Error> {
    if !valid_id(task_id.as_str()) {
        return Err(Error::Invalid("Unknown task id.".into()));
    }
    Ok(app_data
        .join("claude-settings")
        .join(format!("{}.json", task_id.as_str())))
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::*;
    use crate::domain::TaskId;
    use crate::hooks::HookSpec;
    use crate::Error;

    fn tid(s: &str) -> TaskId {
        TaskId::parse(s).unwrap()
    }

    #[test]
    fn json_groups_hooks_by_event_and_matcher_and_omits_empty_sections() {
        let g = GeneratedSettings {
            hooks: vec![
                HookSpec {
                    event: "Stop".into(),
                    matcher: None,
                    command: "a".into(),
                    timeout_s: Some(10),
                },
                HookSpec {
                    event: "PreToolUse".into(),
                    matcher: Some("Bash".into()),
                    command: "b".into(),
                    timeout_s: None,
                },
                HookSpec {
                    event: "PreToolUse".into(),
                    matcher: Some("Bash".into()),
                    command: "c".into(),
                    timeout_s: None,
                },
            ],
            status_line: Some("sl".into()),
            permission_deny: vec![],
            bg_isolation_none: false,
        };
        assert_eq!(
            g.to_json(),
            json!({
                "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "a", "timeout": 10 }] }],
                           "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "b" }, { "type": "command", "command": "c" }] }] },
                "statusLine": { "type": "command", "command": "sl" } })
        );
    }

    #[test]
    fn deny_rules_and_bg_isolation_are_emitted_when_set() {
        let g = GeneratedSettings {
            permission_deny: vec!["Edit(/r/**)".into()],
            bg_isolation_none: true,
            ..Default::default()
        };
        assert_eq!(
            g.to_json(),
            json!({ "permissions": { "deny": ["Edit(/r/**)"] }, "worktree": { "bgIsolation": "none" } })
        );
    }

    #[test]
    fn empty_settings_serialise_to_an_empty_object() {
        assert_eq!(GeneratedSettings::default().to_json(), json!({}));
    }

    #[test]
    fn buffered_hook_command_survives_the_file_verbatim() {
        let cmd =
            r#"sh -c 'IN=$(cat); printf "%s\t%s\t%s\n" "Stop" "$(date -u +%FT%TZ)" "$IN" >> "/ad/events/t-1.jsonl"'"#;
        let g = GeneratedSettings {
            hooks: vec![HookSpec {
                event: "Stop".into(),
                matcher: None,
                command: cmd.into(),
                timeout_s: None,
            }],
            ..Default::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t-1.json");
        g.write(&p).unwrap();
        let back: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(back["hooks"]["Stop"][0]["hooks"][0]["command"], cmd); // quotes, \t, \n and $(…) not altered or expanded
    }

    #[test]
    fn settings_path_validates_the_task_id() {
        let p = settings_path(Path::new("/ad"), &tid("t-abc123")).unwrap();
        assert_eq!(p, PathBuf::from("/ad/claude-settings/t-abc123.json"));
        // serde's transparent Deserialize does not validate, so a bad id can reach here
        let bad: TaskId = serde_json::from_value(json!("../x")).unwrap();
        assert!(matches!(settings_path(Path::new("/ad"), &bad), Err(Error::Invalid(_))));
    }

    #[test]
    fn write_creates_parent_and_sets_mode_0600() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("claude-settings/t-1.json");
        GeneratedSettings::default().write(&p).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&p).unwrap().trim(), "{}");
    }
}
