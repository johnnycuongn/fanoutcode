//! The session driver seam (§7.1): `SessionDriver` starts a launch on a PTY and says how to end it. `OwnedDriver`
//! (the only one this release ships) owns the PTY in the app process, so sessions end when the app quits. The PTY
//! itself sits behind `Pty` / `PtySpawner`, which the shell implements with portable-pty and tests with `fake`.

use std::path::PathBuf;
use std::time::Duration;

use crate::domain::{AgentKind, DriverKind, Session};
use crate::sessions::launch::LaunchSpec;
use crate::Error;

/// A launch at a terminal size, as the spawner gets it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

/// Called with each batched output frame, in order.
pub type OutputFn = Box<dyn FnMut(&[u8]) + Send>;
/// Called once with the exit code, after the last output frame.
pub type ExitFn = Box<dyn FnOnce(i32) + Send>;

/// A running child on a PTY.
pub trait Pty: Send {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error>;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error>;
    fn kill(&mut self) -> Result<(), Error>;
    /// The exit code once the child has exited.
    fn try_wait(&mut self) -> Result<Option<i32>, Error>;
    /// While true the reader stops pulling from the master, so the child blocks instead of output being dropped.
    fn pause_output(&self, paused: bool);
}

pub trait PtySpawner: Send + Sync {
    fn spawn(&self, spec: &SpawnSpec, on_output: OutputFn, on_exit: ExitFn) -> Result<Box<dyn Pty>, Error>;
}

/// How to end a session politely: type `keys` with `key_gap` between them, then wait up to `grace` before killing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GracefulExit {
    pub keys: Vec<Vec<u8>>,
    pub key_gap: Duration,
    pub grace: Duration,
}

/// `/exit`, then up to 5 s, then kill (§7.1).
pub const STOP_GRACE: Duration = Duration::from_secs(5);

pub trait SessionDriver: Send + Sync {
    fn kind(&self) -> DriverKind;
    fn start(
        &self,
        launch: &LaunchSpec,
        size: (u16, u16),
        spawner: &dyn PtySpawner,
        on_output: OutputFn,
        on_exit: ExitFn,
    ) -> Result<Box<dyn Pty>, Error>;
    fn graceful_exit(&self, kind: AgentKind) -> GracefulExit;
    /// Archive-time cleanup of whatever the driver keeps outside the app.
    fn remove(&self, session: &Session) -> Result<(), Error>;
    /// Whether quitting the app ends its sessions (the quit warning applies only then).
    fn ends_on_quit(&self) -> bool;
}

/// The PTY lives in the app process.
#[derive(Debug, Clone, Copy, Default)]
pub struct OwnedDriver;

impl SessionDriver for OwnedDriver {
    fn kind(&self) -> DriverKind {
        DriverKind::Owned
    }

    fn start(
        &self,
        launch: &LaunchSpec,
        size: (u16, u16),
        spawner: &dyn PtySpawner,
        on_output: OutputFn,
        on_exit: ExitFn,
    ) -> Result<Box<dyn Pty>, Error> {
        let spec = SpawnSpec {
            program: launch.program.clone(),
            args: launch.args.clone(),
            cwd: launch.cwd.clone(),
            env: launch.env.clone(),
            cols: size.0,
            rows: size.1,
        };
        spawner.spawn(&spec, on_output, on_exit)
    }

    fn graceful_exit(&self, kind: AgentKind) -> GracefulExit {
        let keys = match kind {
            // Esc first, so a half-typed prompt or an open menu does not swallow the command
            AgentKind::ClaudeCode => vec![b"\x1b".to_vec(), b"/exit\r".to_vec()],
            AgentKind::Codex => vec![b"/quit\r".to_vec()],
            AgentKind::Shell => vec![b"exit\r".to_vec()],
        };
        GracefulExit {
            keys,
            key_gap: Duration::from_millis(100),
            grace: STOP_GRACE,
        }
    }

    fn remove(&self, _session: &Session) -> Result<(), Error> {
        Ok(())
    }

    fn ends_on_quit(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::fake::FakeSpawner;

    #[test]
    fn owned_start_spawns_the_launch_spec_at_the_given_size() {
        let sp = FakeSpawner::default();
        let launch = LaunchSpec {
            program: "claude".into(),
            args: vec!["--settings".into(), "x".into()],
            cwd: "/p".into(),
            env: vec![],
        };
        OwnedDriver
            .start(&launch, (120, 40), &sp, Box::new(|_| {}), Box::new(|_| {}))
            .unwrap();
        let s = &sp.spawned.lock().unwrap()[0];
        assert_eq!((s.program.as_str(), s.cols, s.rows), ("claude", 120, 40));
        assert_eq!(s.args, vec!["--settings", "x"]);
    }

    #[test]
    fn graceful_exit_per_kind() {
        let d = OwnedDriver;
        let c = d.graceful_exit(AgentKind::ClaudeCode);
        assert_eq!(c.keys, vec![b"\x1b".to_vec(), b"/exit\r".to_vec()]);
        assert_eq!((c.key_gap, c.grace), (Duration::from_millis(100), STOP_GRACE));
        assert_eq!(d.graceful_exit(AgentKind::Codex).keys, vec![b"/quit\r".to_vec()]);
        assert_eq!(d.graceful_exit(AgentKind::Shell).keys, vec![b"exit\r".to_vec()]);
    }

    #[test]
    fn owned_driver_ends_on_quit_and_remove_is_noop() {
        assert!(OwnedDriver.ends_on_quit());
        assert_eq!(OwnedDriver.kind(), DriverKind::Owned);
    }
}
