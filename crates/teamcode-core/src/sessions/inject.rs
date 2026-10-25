//! When a session may take a prompt (§7.2): Claude Code is idle between its `SessionStart`/`Stop` hooks and the next
//! prompt, Codex after a quiet spell, a shell always. Prompts that arrive while it works wait in `InjectQueue`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::domain::AgentKind;
use crate::hooks::HookEvent;

/// Codex has no hooks; it counts as idle after this long without output.
pub const CODEX_IDLE_AFTER: Duration = Duration::from_secs(5);
/// The gap between a pasted prompt and the Enter that submits it, so the TUI has taken the paste first.
pub const INJECT_ENTER_DELAY: Duration = Duration::from_millis(50);

/// Whether `inject` typed the prompt now or left it waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOutcome {
    Sent,
    Queued { pending: usize },
}

/// The prompt as one bracketed paste, so its newlines do not submit it early. No Enter: the shell sends `\r`
/// after `INJECT_ENTER_DELAY`.
pub fn frame_prompt(prompt: &str) -> Vec<u8> {
    let mut out = b"\x1b[200~".to_vec();
    out.extend_from_slice(prompt.as_bytes());
    out.extend_from_slice(b"\x1b[201~");
    out
}

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

    /// Hooks only: `SessionStart` and `Stop` make it idle, `UserPromptSubmit` busy; other events change nothing.
    pub fn on_hook(&mut self, event: &HookEvent) {
        if let IdleTracker::Hooks { idle } = self {
            match event {
                HookEvent::SessionStart { .. } | HookEvent::Stop { .. } => *idle = true,
                HookEvent::UserPromptSubmit => *idle = false,
                _ => {}
            }
        }
    }

    /// A prompt was typed: a hooks-driven agent is busy until its next `Stop`.
    pub fn on_injected(&mut self) {
        if let IdleTracker::Hooks { idle } = self {
            *idle = false;
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

    /// Puts a prompt back at the head (a write failed).
    pub fn push_front(&mut self, prompt: String) {
        self.0.push_front(prompt);
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::hooks::HookEvent;

    #[test]
    fn claude_is_idle_after_session_start_or_stop_and_working_after_prompt() {
        let mut t = IdleTracker::for_kind(AgentKind::ClaudeCode);
        let now = Instant::now();
        assert!(!t.is_idle(now));
        t.on_hook(&HookEvent::SessionStart {
            session_id: "s".into(),
            transcript_path: "/t".into(),
        });
        assert!(t.is_idle(now));
        t.on_hook(&HookEvent::UserPromptSubmit);
        assert!(!t.is_idle(now));
        t.on_hook(&HookEvent::Stop {
            last_assistant_message: "".into(),
            background_tasks: 0,
        });
        assert!(t.is_idle(now));
        t.on_injected();
        assert!(!t.is_idle(now));
    }

    #[test]
    fn codex_is_idle_after_five_quiet_seconds() {
        let mut t = IdleTracker::for_kind(AgentKind::Codex);
        let t0 = Instant::now();
        t.on_output(t0);
        assert!(!t.is_idle(t0 + Duration::from_millis(4999)));
        assert!(t.is_idle(t0 + Duration::from_secs(5)));
    }

    #[test]
    fn shell_is_always_idle() {
        assert!(IdleTracker::for_kind(AgentKind::Shell).is_idle(Instant::now()));
    }

    #[test]
    fn frame_uses_bracketed_paste_without_enter() {
        assert_eq!(frame_prompt("a\nb"), b"\x1b[200~a\nb\x1b[201~".to_vec());
    }
}
