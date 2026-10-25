//! Hook events (§7.3, contracts "Hook event log", A1, A3, A18): the per-task event log, its typed events and the
//! commands that write it. Each hook in a task's generated settings file appends one line,
//! `EventName<TAB>rfc3339<TAB>{claude payload json}`, to `<app_data>/events/<task-id>.jsonl`.

pub mod artifact;
pub mod attribution;
pub mod cursor;
pub mod feed;
pub mod parse;
pub mod plan_steps;
pub mod specs;
pub mod subscribers;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::TaskId;

pub use specs::{
    hook_command, hook_specs, sh_single_quote, status_line_command, user_status_line, HookSpec, HOOK_EVENTS,
};

/// One parsed line of a task's event log. `seq` is the 1-based line number; `agent_id` is the payload's subagent id,
/// normalised (no `agent-` prefix), `None` for the main agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct TaskEvent {
    pub seq: u64,
    pub at: DateTime<Utc>,
    pub task_id: TaskId,
    pub agent_id: Option<String>,
    pub event: HookEvent,
}

/// What a hook reported. Internally tagged (A1): `{ "type": "Stop", "last_assistant_message": "…", … }`. The tag is
/// the IPC shape only; the parser picks the variant from the log line's first field, never from the payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum HookEvent {
    SessionStart {
        session_id: String,
        transcript_path: PathBuf,
    },
    UserPromptSubmit,
    PreToolUse {
        tool: String,
        input: serde_json::Value,
    },
    PostToolUse {
        tool: String,
        input: serde_json::Value,
        response: serde_json::Value,
    },
    SubagentStart {
        agent_id: String,
        agent_type: String,
        description: Option<String>,
    },
    SubagentStop {
        agent_id: String,
    },
    PermissionRequest {
        tool: String,
    },
    Notification {
        message: String,
    },
    Stop {
        last_assistant_message: String,
        background_tasks: usize,
    },
    StopFailure {
        message: String,
    },
    StatusLine {
        cost_usd: f64,
        input_tokens: u64,
        output_tokens: u64,
        context_used_pct: f32,
    },
    Unknown {
        name: String,
    },
}

/// `<app_data>/events/<task-id>.jsonl`. The id was validated when it was parsed, so it is safe in a path.
pub fn event_log_path(app_data: &Path, task_id: &TaskId) -> PathBuf {
    app_data.join("events").join(format!("{}.jsonl", task_id.as_str()))
}
