//! The event-log line parser (Review Focus 1): `EventName<TAB>rfc3339<TAB>payload`. A partial tail waits for its
//! newline; a payload that is not JSON is kept as `Unknown` and never stops the tailer.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::{HookEvent, TaskEvent};
use crate::domain::TaskId;
use crate::{Error, Result};

/// A subagent id without Claude Code's `agent-` prefix (`agent-a1` → `a1`).
pub fn normalize_agent_id(raw: &str) -> String {
    raw.strip_prefix("agent-").unwrap_or(raw).to_owned()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or("").to_owned()
}

fn field(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or(Value::Null)
}

fn opt_text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// A task id field; absent, null or not a valid id is `None`.
fn task_id(v: &Value, key: &str) -> Option<TaskId> {
    v.get(key).and_then(Value::as_str).and_then(|s| TaskId::parse(s).ok())
}

/// The event the line's first field names, read from `payload`. Missing fields default to `""`, `Null` or `0`.
fn event_from(name: &str, p: &Value) -> HookEvent {
    match name {
        "SessionStart" => HookEvent::SessionStart {
            session_id: text(p, "session_id"),
            transcript_path: text(p, "transcript_path").into(),
        },
        "UserPromptSubmit" => HookEvent::UserPromptSubmit,
        "PreToolUse" => HookEvent::PreToolUse {
            tool: text(p, "tool_name"),
            input: field(p, "tool_input"),
        },
        "PostToolUse" => HookEvent::PostToolUse {
            tool: text(p, "tool_name"),
            input: field(p, "tool_input"),
            response: field(p, "tool_response"),
        },
        "SubagentStart" => HookEvent::SubagentStart {
            agent_id: normalize_agent_id(&text(p, "agent_id")),
            agent_type: text(p, "agent_type"),
            description: None,
        },
        "SubagentStop" => HookEvent::SubagentStop {
            agent_id: normalize_agent_id(&text(p, "agent_id")),
        },
        "PermissionRequest" => HookEvent::PermissionRequest {
            tool: text(p, "tool_name"),
        },
        "Notification" => HookEvent::Notification {
            message: text(p, "message"),
        },
        "Stop" => HookEvent::Stop {
            last_assistant_message: text(p, "last_assistant_message"),
            background_tasks: p.get("background_tasks").and_then(Value::as_array).map_or(0, Vec::len),
        },
        "StopFailure" => HookEvent::StopFailure {
            message: ["last_assistant_message", "error_details", "error"]
                .iter()
                .map(|k| text(p, k))
                .find(|m| !m.is_empty())
                .unwrap_or_default(),
        },
        "StatusLine" => {
            let window = |k: &str| p.pointer(&format!("/context_window/{k}"));
            HookEvent::StatusLine {
                cost_usd: p.pointer("/cost/total_cost_usd").and_then(Value::as_f64).unwrap_or(0.0),
                input_tokens: window("total_input_tokens").and_then(Value::as_u64).unwrap_or(0),
                output_tokens: window("total_output_tokens").and_then(Value::as_u64).unwrap_or(0),
                context_used_pct: window("used_percentage").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            }
        }
        "ClaimDenied" => HookEvent::ClaimDenied {
            repo: text(p, "repo").into(),
            path: opt_text(p, "path"),
            command: opt_text(p, "command"),
            cwd: opt_text(p, "cwd").map(PathBuf::from),
            other_task: task_id(p, "other_task"),
        },
        "ClaimNoted" => HookEvent::ClaimNoted {
            repo: text(p, "repo").into(),
            path: opt_text(p, "path"),
            command: opt_text(p, "command"),
            other_task: task_id(p, "other_task"),
        },
        "CommandRan" => match serde_json::from_value(field(p, "kind")) {
            Ok(kind) => HookEvent::CommandRan {
                repo: text(p, "repo").into(),
                command: text(p, "command"),
                kind,
                paths: p
                    .get("paths")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                    .unwrap_or_default(),
                other_task: task_id(p, "other_task"),
            },
            Err(_) => HookEvent::Unknown { name: name.to_owned() },
        },
        "OverlapDecided" => HookEvent::OverlapDecided {
            repo: text(p, "repo").into(),
            path: text(p, "path"),
            other_task: task_id(p, "other_task"),
            decision: text(p, "decision"),
        },
        "OverlapNoted" => match task_id(p, "other_task") {
            Some(other_task) => HookEvent::OverlapNoted {
                repo: text(p, "repo").into(),
                path: text(p, "path"),
                other_task,
                direction: text(p, "direction"),
            },
            None => HookEvent::Unknown { name: name.to_owned() },
        },
        other => HookEvent::Unknown { name: other.to_owned() },
    }
}

/// One log line. Fewer than three tab-separated fields, or a bad timestamp, is `Invalid`; a payload that is not
/// JSON is kept as `Unknown { name }`.
pub fn parse_line(task_id: &TaskId, seq: u64, line: &str) -> Result<TaskEvent> {
    let mut parts = line.splitn(3, '\t');
    let (Some(name), Some(at), Some(payload)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(Error::Invalid(format!(
            "Event log line {seq} has no event name, time and payload"
        )));
    };
    let at: DateTime<Utc> = DateTime::parse_from_rfc3339(at)
        .map_err(|e| Error::Invalid(format!("Event log line {seq} has a bad time: {e}")))?
        .with_timezone(&Utc);
    let (event, agent_id) = match serde_json::from_str::<Value>(payload) {
        Ok(p) => (
            event_from(name, &p),
            p.get("agent_id").and_then(Value::as_str).map(normalize_agent_id),
        ),
        Err(_) => (HookEvent::Unknown { name: name.to_owned() }, None),
    };
    Ok(TaskEvent {
        seq,
        at,
        task_id: task_id.clone(),
        agent_id,
        event,
    })
}

/// Drains every `\n`-terminated line from `buf` (without the newline, `\r` trimmed) and leaves the partial tail.
pub fn split_complete_lines(buf: &mut Vec<u8>) -> Vec<String> {
    let Some(last) = buf.iter().rposition(|&b| b == b'\n') else {
        return Vec::new();
    };
    let complete: Vec<u8> = buf.drain(..=last).collect();
    complete[..complete.len() - 1]
        .split(|&b| b == b'\n')
        .map(|l| String::from_utf8_lossy(l).trim_end_matches('\r').to_owned())
        .collect()
}

/// Every complete line of a task's log, `seq` = the 1-based line number. A missing file is empty; a line that does
/// not parse is skipped (its number is still used).
pub fn read_all(path: &Path, task_id: &TaskId) -> Result<Vec<TaskEvent>> {
    let mut buf = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(split_complete_lines(&mut buf)
        .iter()
        .enumerate()
        .filter_map(|(i, l)| parse_line(task_id, i as u64 + 1, l).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use chrono::SecondsFormat;

    use super::*;

    const FIXTURE: &str = include_str!("fixtures/events.jsonl");

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    fn line(n: usize) -> TaskEvent {
        parse_line(&tid(), n as u64, FIXTURE.lines().nth(n - 1).unwrap()).unwrap()
    }

    #[test]
    fn session_start_parses_session_and_transcript() {
        let e = line(1);
        assert_eq!(e.seq, 1);
        assert_eq!(e.at.to_rfc3339_opts(SecondsFormat::Secs, true), "2026-10-24T05:12:33Z");
        assert!(
            matches!(e.event, HookEvent::SessionStart { ref session_id, ref transcript_path }
            if session_id == "9549b821-8d80-4fc9-83b6-416d1d34d802" && transcript_path.ends_with("9549b821-8d80-4fc9-83b6-416d1d34d802.jsonl"))
        );
    }

    #[test]
    fn post_tool_use_keeps_tool_input_response_and_agent_id() {
        let e = line(5);
        assert_eq!(e.agent_id.as_deref(), Some("a1"));
        let HookEvent::PostToolUse { tool, input, response } = e.event else {
            panic!()
        };
        assert_eq!(tool, "Edit");
        assert_eq!(input["file_path"], "/Users/j/app/src/Board.tsx");
        assert_eq!(response["filePath"], "/Users/j/app/src/Board.tsx");
    }

    #[test]
    fn subagent_ids_are_normalised() {
        assert!(
            matches!(line(4).event, HookEvent::SubagentStart { ref agent_id, ref agent_type, description: None } if agent_id == "a1" && agent_type == "frontend")
        );
    }

    #[test]
    fn stop_counts_background_tasks() {
        assert!(matches!(
            line(9).event,
            HookEvent::Stop {
                background_tasks: 0,
                ..
            }
        ));
        assert!(matches!(
            line(10).event,
            HookEvent::Stop {
                background_tasks: 1,
                ..
            }
        ));
    }

    #[test]
    fn stop_failure_message_prefers_last_assistant_message() {
        assert!(
            matches!(line(11).event, HookEvent::StopFailure { ref message } if message == "API Error: Rate limit reached")
        );
    }

    #[test]
    fn status_line_maps_cost_and_context_fields() {
        assert_eq!(
            line(12).event,
            HookEvent::StatusLine {
                cost_usd: 0.01234,
                input_tokens: 15500,
                output_tokens: 1200,
                context_used_pct: 8.0
            }
        );
    }

    #[test]
    fn unknown_event_names_are_preserved() {
        assert!(matches!(line(13).event, HookEvent::Unknown { ref name } if name == "TeammateIdle"));
    }

    #[test]
    fn a_payload_that_is_not_json_is_preserved_as_unknown() {
        let e = parse_line(&tid(), 1, "Stop\t2026-10-24T05:15:00Z\t{\"half").unwrap();
        assert!(matches!(e.event, HookEvent::Unknown { ref name } if name == "Stop"));
    }

    #[test]
    fn parses_command_ran() {
        let a = TaskId::parse("t-a1").unwrap();
        let line = "CommandRan\t2026-10-25T10:02:00Z\t{\"repo\":\"/r\",\"command\":\"git stash\",\"kind\":\"Discards\",\"paths\":[\"src/a.ts\"],\"other_task\":\"t-b2\"}";
        assert_eq!(
            parse_line(&a, 1, line).unwrap().event,
            HookEvent::CommandRan {
                repo: "/r".into(),
                command: "git stash".into(),
                kind: crate::overlap::CommandKind::Discards,
                paths: vec!["src/a.ts".into()],
                other_task: Some(TaskId::parse("t-b2").unwrap()),
            }
        );
        // a kind it cannot read is not a run
        let bad = "CommandRan\t2026-10-25T10:02:00Z\t{\"repo\":\"/r\",\"command\":\"git stash\",\"kind\":\"Eats\"}";
        assert!(matches!(
            parse_line(&a, 2, bad).unwrap().event,
            HookEvent::Unknown { .. }
        ));
    }

    #[test]
    fn parses_claim_denied_and_overlap_lines() {
        let a = TaskId::parse("t-a1").unwrap();
        let b = TaskId::parse("t-b2").unwrap();
        let line = "ClaimDenied\t2026-10-25T10:00:00Z\t{\"repo\":\"/r\",\"path\":\"src/a.ts\",\"other_task\":\"t-b2\"}";
        let ev = parse_line(&a, 1, line).unwrap();
        assert_eq!(
            ev.event,
            HookEvent::ClaimDenied {
                repo: "/r".into(),
                path: Some("src/a.ts".into()),
                command: None,
                cwd: None,
                other_task: Some(b.clone()),
            }
        );
        let line = "ClaimDenied\t2026-10-25T10:00:00Z\t{\"repo\":\"/r\",\"command\":\"git add a.ts\",\"cwd\":\"/r/src\",\"other_task\":null}";
        assert!(matches!(parse_line(&a, 2, line).unwrap().event,
            HookEvent::ClaimDenied { cwd: Some(ref d), .. } if d == Path::new("/r/src")));
        let line =
            "ClaimDenied\t2026-10-25T10:00:00Z\t{\"repo\":\"/r\",\"command\":\"git add -A\",\"other_task\":null}";
        assert!(matches!(parse_line(&a, 2, line).unwrap().event,
            HookEvent::ClaimDenied { path: None, command: Some(ref c), other_task: None, .. } if c == "git add -A"));
        let line = "ClaimNoted\t2026-10-25T10:00:30Z\t{\"repo\":\"/r\",\"path\":\"src/a.ts\",\"command\":null,\"other_task\":\"t-b2\"}";
        assert_eq!(
            parse_line(&a, 2, line).unwrap().event,
            HookEvent::ClaimNoted {
                repo: "/r".into(),
                path: Some("src/a.ts".into()),
                command: None,
                other_task: Some(b.clone()),
            }
        );
        let line = "OverlapDecided\t2026-10-25T10:01:00Z\t{\"repo\":\"/r\",\"path\":\"src/a.ts\",\"other_task\":\"t-b2\",\"decision\":\"OnTopFile\"}";
        assert_eq!(
            parse_line(&a, 3, line).unwrap().event,
            HookEvent::OverlapDecided {
                repo: "/r".into(),
                path: "src/a.ts".into(),
                other_task: Some(b.clone()),
                decision: "OnTopFile".into(),
            }
        );
        let line = "OverlapNoted\t2026-10-25T10:02:00Z\t{\"repo\":\"/r\",\"path\":\"src/a.ts\",\"other_task\":\"t-b2\",\"direction\":\"out\"}";
        assert_eq!(
            parse_line(&a, 4, line).unwrap().event,
            HookEvent::OverlapNoted {
                repo: "/r".into(),
                path: "src/a.ts".into(),
                other_task: b,
                direction: "out".into(),
            }
        );
        // a bad other_task id on a noted line is no event of ours
        let line = "OverlapNoted\t2026-10-25T10:02:00Z\t{\"repo\":\"/r\",\"path\":\"a\",\"other_task\":\"../x\"}";
        assert!(
            matches!(parse_line(&a, 5, line).unwrap().event, HookEvent::Unknown { ref name } if name == "OverlapNoted")
        );
        let line = "Whatever\t2026-10-25T10:02:00Z\t{}";
        assert!(matches!(
            parse_line(&a, 6, line).unwrap().event,
            HookEvent::Unknown { .. }
        ));
    }

    #[test]
    fn a_line_without_two_tabs_is_invalid() {
        assert!(matches!(parse_line(&tid(), 1, "garbage"), Err(Error::Invalid(_))));
    }

    #[test]
    fn split_complete_lines_keeps_the_partial_tail() {
        let mut buf = b"a\nb\nc".to_vec();
        assert_eq!(split_complete_lines(&mut buf), vec!["a".to_string(), "b".to_string()]);
        assert_eq!(buf, b"c");
    }

    #[test]
    fn read_all_numbers_lines_from_one() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("t-1.jsonl");
        std::fs::write(&p, FIXTURE).unwrap();
        let all = read_all(&p, &tid()).unwrap();
        assert_eq!(all.len(), 13);
        assert_eq!(all[12].seq, 13);
    }
}
