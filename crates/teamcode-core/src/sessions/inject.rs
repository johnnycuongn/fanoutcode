//! When a session may take a prompt (§7.2): Claude Code is idle between its `SessionStart`/`Stop` hooks and the next
//! prompt, Codex after a quiet spell, a shell always. Prompts that arrive while it works wait in `InjectQueue`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::domain::AgentKind;

/// Codex has no hooks; it counts as idle after this long without output.
pub const CODEX_IDLE_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdleTracker {
    Hooks {
        idle: bool,
    },
    Quiet {
        last_output: Option<Instant>,
        after: Duration,
    },
    Always,
}

impl IdleTracker {
    /// ClaudeCode → `Hooks { idle: false }`; Codex → `Quiet { None, CODEX_IDLE_AFTER }`; Shell → `Always`.
    pub fn for_kind(kind: AgentKind) -> Self {
        match kind {
            AgentKind::ClaudeCode => IdleTracker::Hooks { idle: false },
            AgentKind::Codex => IdleTracker::Quiet {
                last_output: None,
                after: CODEX_IDLE_AFTER,
            },
            AgentKind::Shell => IdleTracker::Always,
        }
    }

    /// Quiet only: output restarts the quiet clock.
    pub fn on_output(&mut self, now: Instant) {
        if let IdleTracker::Quiet { last_output, .. } = self {
            *last_output = Some(now);
        }
    }

    /// A Quiet tracker that has seen no output yet is still starting up, so it is not idle.
    pub fn is_idle(&self, now: Instant) -> bool {
        match self {
            IdleTracker::Hooks { idle } => *idle,
            IdleTracker::Quiet { last_output, after } => {
                last_output.is_some_and(|t| now.saturating_duration_since(t) >= *after)
            }
            IdleTracker::Always => true,
        }
    }
}

/// Prompts that will send when the session is idle, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InjectQueue(VecDeque<String>);

impl InjectQueue {
    /// Queues `prompt`; returns how many are pending.
    pub fn push(&mut self, prompt: String) -> usize {
        self.0.push_back(prompt);
        self.0.len()
    }

    pub fn pop_front(&mut self) -> Option<String> {
        self.0.pop_front()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}
