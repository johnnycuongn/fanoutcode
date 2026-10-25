//! Test doubles for the PTY seam: `FakeSpawner` records what it spawned, `FakePty` records writes, resizes, kills and
//! pauses and lets a test feed output and an exit code through its `FakePtyHandle`. Compiled for tests and for the
//! `test-support` feature, so the shell's tests can use them too.

use std::cell::Cell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::sessions::driver::{ExitFn, OutputFn, Pty, PtySpawner, SpawnSpec};
use crate::Error;

thread_local! {
    /// The fake time this thread's `FakeClock` has slept through; `FakePty::try_wait` reads it.
    static FAKE_ELAPSED: Cell<Duration> = const { Cell::new(Duration::ZERO) };
}

/// The fake time elapsed on this thread.
pub fn fake_elapsed() -> Duration {
    FAKE_ELAPSED.with(Cell::get)
}

#[derive(Default)]
struct FakeState {
    writes: Vec<Vec<u8>>,
    resizes: Vec<(u16, u16)>,
    killed: bool,
    paused: bool,
    exit_after: Option<(Duration, i32)>,
    on_output: Option<OutputFn>,
    on_exit: Option<ExitFn>,
}

/// The test's side of a `FakePty`.
#[derive(Clone, Default)]
pub struct FakePtyHandle(Arc<Mutex<FakeState>>);

impl FakePtyHandle {
    fn with<R>(&self, f: impl FnOnce(&mut FakeState) -> R) -> R {
        f(&mut self.0.lock().expect("fake pty lock"))
    }

    pub fn writes(&self) -> Vec<Vec<u8>> {
        self.with(|s| s.writes.clone())
    }

    pub fn killed(&self) -> bool {
        self.with(|s| s.killed)
    }

    pub fn paused(&self) -> bool {
        self.with(|s| s.paused)
    }

    pub fn last_resize(&self) -> Option<(u16, u16)> {
        self.with(|s| s.resizes.last().copied())
    }

    pub fn resizes(&self) -> Vec<(u16, u16)> {
        self.with(|s| s.resizes.clone())
    }

    /// `try_wait` returns `code` once this thread's fake clock has passed `after`.
    pub fn exit_after(&self, after: Duration, code: i32) {
        self.with(|s| s.exit_after = Some((after, code)));
    }

    /// Delivers `bytes` to the spawner's `on_output`, as the reader would.
    pub fn feed(&self, bytes: &[u8]) {
        let f = self.with(|s| s.on_output.take());
        if let Some(mut f) = f {
            f(bytes);
            self.with(|s| s.on_output = Some(f));
        }
    }

    /// Calls the spawner's `on_exit` once, as the reader would.
    pub fn exit(&self, code: i32) {
        if let Some(f) = self.with(|s| s.on_exit.take()) {
            f(code);
        }
    }
}

pub struct FakePty(FakePtyHandle);

impl FakePty {
    pub fn pair() -> (FakePty, FakePtyHandle) {
        let h = FakePtyHandle::default();
        (FakePty(h.clone()), h)
    }
}

impl Pty for FakePty {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.0.with(|s| s.writes.push(bytes.to_vec()));
        Ok(())
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.0.with(|s| s.resizes.push((cols, rows)));
        Ok(())
    }

    fn kill(&mut self) -> Result<(), Error> {
        self.0.with(|s| s.killed = true);
        Ok(())
    }

    fn try_wait(&mut self) -> Result<Option<i32>, Error> {
        let now = fake_elapsed();
        Ok(self.0.with(|s| match s.exit_after {
            Some((after, code)) if now >= after => Some(code),
            _ if s.killed => Some(-9),
            _ => None,
        }))
    }

    fn pause_output(&self, paused: bool) {
        self.0.with(|s| s.paused = paused);
    }
}

/// Records every spawn and hands back a `FakePty` whose handle is kept in `ptys`.
#[derive(Clone, Default)]
pub struct FakeSpawner {
    pub spawned: Arc<Mutex<Vec<SpawnSpec>>>,
    pub ptys: Arc<Mutex<Vec<FakePtyHandle>>>,
}

impl PtySpawner for FakeSpawner {
    fn spawn(&self, spec: &SpawnSpec, on_output: OutputFn, on_exit: ExitFn) -> Result<Box<dyn Pty>, Error> {
        self.spawned.lock().expect("spawned lock").push(spec.clone());
        let (pty, handle) = FakePty::pair();
        handle.with(|s| {
            s.on_output = Some(on_output);
            s.on_exit = Some(on_exit);
        });
        self.ptys.lock().expect("ptys lock").push(handle);
        Ok(Box::new(pty))
    }
}
