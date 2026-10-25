//! The graceful end (§7.1): type the agent's exit keys, wait up to the grace period for the process to leave, then
//! kill it. Runs on a worker thread in the shell, never on the IPC thread, because it sleeps.

use std::time::{Duration, Instant};

use crate::sessions::driver::{GracefulExit, Pty};

/// Time for the stop sequence, so tests run it instantly.
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
    fn sleep(&self, d: Duration);
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// How often the process is checked during the grace period.
pub const STOP_POLL: Duration = Duration::from_millis(100);
/// How long a killed process gets to report its code.
const KILL_WAIT: Duration = Duration::from_secs(1);

/// The exit code; `-9` when the process had to be killed.
pub fn run(exit: &GracefulExit, pty: &mut dyn Pty, clock: &dyn Clock) -> i32 {
    if let Ok(Some(code)) = pty.try_wait() {
        return code;
    }
    for key in &exit.keys {
        // a failed write means the child is going anyway; the poll below finds out
        let _ = pty.write(key);
        clock.sleep(exit.key_gap);
    }
    if let Some(code) = poll(pty, clock, exit.grace) {
        return code;
    }
    let _ = pty.kill();
    let _ = poll(pty, clock, KILL_WAIT);
    -9
}

/// Polls `try_wait` every `STOP_POLL` until `limit` has passed.
fn poll(pty: &mut dyn Pty, clock: &dyn Clock, limit: Duration) -> Option<i32> {
    let start = clock.now();
    loop {
        if let Ok(Some(code)) = pty.try_wait() {
            return Some(code);
        }
        if clock.now().saturating_duration_since(start) >= limit {
            return None;
        }
        clock.sleep(STOP_POLL);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::domain::AgentKind;
    use crate::sessions::driver::{OwnedDriver, SessionDriver, STOP_GRACE};
    use crate::sessions::fake::{FakeClock, FakePty};

    #[test]
    fn types_keys_with_gap_and_returns_when_process_exits_within_grace() {
        let clock = FakeClock::default();
        let (mut pty, h) = FakePty::pair();
        h.exit_after(Duration::from_secs(1), 0);
        let code = run(&OwnedDriver.graceful_exit(AgentKind::ClaudeCode), &mut pty, &clock);
        assert_eq!(code, 0);
        assert_eq!(h.writes(), vec![b"\x1b".to_vec(), b"/exit\r".to_vec()]);
        assert!(!h.killed());
        assert!(clock.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn kills_after_five_seconds_when_process_ignores_exit() {
        let clock = FakeClock::default();
        let (mut pty, h) = FakePty::pair(); // never exits
        let code = run(&OwnedDriver.graceful_exit(AgentKind::ClaudeCode), &mut pty, &clock);
        assert_eq!(code, -9);
        assert!(h.killed());
        assert!(clock.elapsed() >= STOP_GRACE && clock.elapsed() < STOP_GRACE + Duration::from_secs(1));
    }

    #[test]
    fn already_exited_process_returns_its_code_without_typing() {
        let clock = FakeClock::default();
        let (mut pty, h) = FakePty::pair();
        h.exit_after(Duration::ZERO, 2);
        assert_eq!(run(&OwnedDriver.graceful_exit(AgentKind::Shell), &mut pty, &clock), 2);
        assert!(h.writes().is_empty());
    }
}
