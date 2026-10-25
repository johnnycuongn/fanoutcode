//! The hook commands a task's generated settings file carries (A3), and the status-line forwarder (§10.6). Plan 04
//! writes them into the per-task settings file; Plan 10 adds its guard hook beside them.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::event_log_path;
use crate::domain::TaskId;

/// Every Claude Code hook event the app logs, in the order `hook_specs` returns them.
pub const HOOK_EVENTS: [&str; 10] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "SubagentStart",
    "SubagentStop",
    "PermissionRequest",
    "Notification",
    "Stop",
    "StopFailure",
];

/// Claude Code's hook `timeout`, in seconds.
const HOOK_TIMEOUT_S: u32 = 5;

/// One hook entry of a generated settings file (A3). Plans 04 and 10 import it from here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct HookSpec {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
    pub timeout_s: Option<u32>,
}

/// One spec per `HOOK_EVENTS` entry, each appending to the task's event log.
pub fn hook_specs(task_id: &TaskId, app_data: &Path) -> Vec<HookSpec> {
    let log = event_log_path(app_data, task_id);
    HOOK_EVENTS
        .iter()
        .map(|event| HookSpec {
            event: (*event).to_owned(),
            matcher: None,
            command: hook_command(event, &log),
            timeout_s: Some(HOOK_TIMEOUT_S),
        })
        .collect()
}

/// The path as it sits inside the command's outer single quotes: each `'` becomes `'\''`.
fn in_single_quotes(s: &str) -> String {
    s.replace('\'', r"'\''")
}

/// A3's buffered one-write line: the payload is read whole before the single `printf`, so parallel subagent hooks
/// append whole lines.
pub fn hook_command(event: &str, log_path: &Path) -> String {
    let log = in_single_quotes(&log_path.display().to_string());
    format!(r#"sh -c 'IN=$(cat); printf "%s\t%s\t%s\n" "{event}" "$(date -u +%FT%TZ)" "$IN" >> "{log}"'"#)
}

/// `s` as one single-quoted shell word.
pub fn sh_single_quote(s: &str) -> String {
    format!("'{}'", in_single_quotes(s))
}

/// The status-line command: appends one `StatusLine` line, then pipes the same input to the developer's own
/// status-line command (positional `$1`), so their display is unchanged. Prints nothing when they have none.
pub fn status_line_command(log_path: &Path, user_command: Option<&str>) -> String {
    let log = in_single_quotes(&log_path.display().to_string());
    let user = sh_single_quote(user_command.unwrap_or(""));
    format!(
        r#"sh -c 'IN=$(cat); printf "%s\t%s\t%s\n" StatusLine "$(date -u +%FT%TZ)" "$IN" >> "{log}"; if [ -n "$1" ]; then printf "%s" "$IN" | sh -c "$1"; fi; exit 0' fanoutcode-statusline {user}"#
    )
}

/// The developer's own status-line command from their Claude Code settings JSON, only for `"type": "command"`.
pub fn user_status_line(user_settings_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(user_settings_json).ok()?;
    let sl = v.get("statusLine")?;
    if sl.get("type")?.as_str()? != "command" {
        return None;
    }
    sl.get("command")?.as_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::*;
    use crate::hooks::HookEvent;

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    #[test]
    fn hook_specs_cover_every_event_once() {
        let specs = hook_specs(&tid(), Path::new("/tmp/x"));
        assert_eq!(
            specs.iter().map(|s| s.event.as_str()).collect::<Vec<_>>(),
            HOOK_EVENTS.to_vec()
        );
        assert!(specs.iter().all(|s| s.matcher.is_none() && s.timeout_s == Some(5)));
    }

    #[test]
    fn session_start_command_is_the_buffered_one_write_line() {
        let specs = hook_specs(&tid(), Path::new("/tmp/x"));
        assert_eq!(
            specs[0].command,
            r#"sh -c 'IN=$(cat); printf "%s\t%s\t%s\n" "SessionStart" "$(date -u +%FT%TZ)" "$IN" >> "/tmp/x/events/t-1.jsonl"'"#
        );
    }

    #[test]
    fn a_quote_in_the_log_path_is_escaped_for_the_single_quotes() {
        let c = hook_command("Stop", Path::new("/tmp/it's/events/t-1.jsonl"));
        assert!(c.ends_with(r#">> "/tmp/it'\''s/events/t-1.jsonl"'"#));
    }

    #[test]
    fn hook_events_serialise_tagged_by_type() {
        let stop = HookEvent::Stop {
            last_assistant_message: "x".into(),
            background_tasks: 0,
        };
        assert_eq!(
            serde_json::to_value(&stop).unwrap(),
            json!({"type": "Stop", "last_assistant_message": "x", "background_tasks": 0})
        );
        assert_eq!(
            serde_json::to_value(&HookEvent::Unknown {
                name: "TeammateIdle".into()
            })
            .unwrap(),
            json!({"type": "Unknown", "name": "TeammateIdle"})
        );
        assert_eq!(
            serde_json::to_value(&HookEvent::UserPromptSubmit).unwrap(),
            json!({"type": "UserPromptSubmit"})
        );
    }

    #[test]
    fn status_line_command_logs_then_forwards() {
        let c = status_line_command(Path::new("/tmp/x/events/t-1.jsonl"), Some("jq -r '.model.id'"));
        assert_eq!(
            c,
            r#"sh -c 'IN=$(cat); printf "%s\t%s\t%s\n" StatusLine "$(date -u +%FT%TZ)" "$IN" >> "/tmp/x/events/t-1.jsonl"; if [ -n "$1" ]; then printf "%s" "$IN" | sh -c "$1"; fi; exit 0' fanoutcode-statusline 'jq -r '\''.model.id'\'''"#
        );
    }

    #[test]
    fn status_line_without_user_command_prints_nothing() {
        let c = status_line_command(Path::new("/tmp/x/events/t-1.jsonl"), None);
        assert!(c.ends_with("fanoutcode-statusline ''"));
    }

    #[test]
    fn user_status_line_reads_only_command_type() {
        assert_eq!(
            user_status_line(r#"{"statusLine":{"type":"command","command":"~/.claude/sl.sh","padding":2}}"#),
            Some("~/.claude/sl.sh".into())
        );
        assert_eq!(user_status_line(r#"{"statusLine":{"type":"agent"}}"#), None);
        assert_eq!(user_status_line("not json"), None);
    }

    #[test]
    fn every_hook_event_variant_round_trips() {
        // internally tagged variants fail only at runtime (core rules)
        for e in [
            HookEvent::SessionStart {
                session_id: "s".into(),
                transcript_path: "/t.jsonl".into(),
            },
            HookEvent::UserPromptSubmit,
            HookEvent::PreToolUse {
                tool: "Bash".into(),
                input: json!({"command": "ls"}),
            },
            HookEvent::PostToolUse {
                tool: "Edit".into(),
                input: json!({}),
                response: json!(null),
            },
            HookEvent::SubagentStart {
                agent_id: "a1".into(),
                agent_type: "frontend".into(),
                description: Some("Step 1 · frontend: x".into()),
            },
            HookEvent::SubagentStop { agent_id: "a1".into() },
            HookEvent::PermissionRequest { tool: "Bash".into() },
            HookEvent::Notification { message: "m".into() },
            HookEvent::Stop {
                last_assistant_message: "x".into(),
                background_tasks: 1,
            },
            HookEvent::StopFailure { message: "e".into() },
            HookEvent::StatusLine {
                cost_usd: 0.5,
                input_tokens: 1,
                output_tokens: 2,
                context_used_pct: 8.0,
            },
            HookEvent::ClaimDenied {
                repo: "/r".into(),
                path: Some("a.ts".into()),
                command: None,
                other_task: Some(crate::domain::TaskId::parse("t-b2").unwrap()),
            },
            HookEvent::ClaimNoted {
                repo: "/r".into(),
                path: Some("a.ts".into()),
                command: None,
                other_task: None,
            },
            HookEvent::OverlapDecided {
                repo: "/r".into(),
                path: "a.ts".into(),
                other_task: None,
                decision: "Hold".into(),
            },
            HookEvent::OverlapNoted {
                repo: "/r".into(),
                path: "a.ts".into(),
                other_task: crate::domain::TaskId::parse("t-b2").unwrap(),
                direction: "in".into(),
            },
            HookEvent::Unknown { name: "X".into() },
        ] {
            let back: HookEvent = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
            assert_eq!(back, e);
        }
    }
}
