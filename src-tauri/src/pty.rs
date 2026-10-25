//! The PTY behind the core's `PtySpawner` seam, with portable-pty. Per session: a reader thread (read → batch → a
//! full frame goes out at once), a flusher thread that parks while the batch is empty and otherwise sends the open
//! frame once `Batcher::INTERVAL` after its first byte (so an idle session costs no wakeups), and a waiter thread
//! that reports the exit code once, after the last frame. `pause_output(true)` closes a gate the reader checks
//! before every read, so flow control blocks the child instead of dropping output.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use livespec_core::shell_env::login_shell_path;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use teamcode_core::sessions::{Batcher, ExitFn, OutputFn, OutputSink, Pty, PtySpawner, SpawnSpec};
use teamcode_core::Error;

/// How long the exit report waits for the output after the child exits (a background job may hold the PTY open).
const DRAIN_WAIT: Duration = Duration::from_millis(500);
const READ_CHUNK: usize = 16 * 1024;

/// Terminal output on a channel. It is sent as raw bytes (an `ArrayBuffer` in the webview), not a JSON array; the
/// generated type says `number[]`, and `terminalStore.toBytes` accepts both.
#[derive(Debug, Clone, specta::Type)]
#[specta(transparent)]
pub struct TerminalBytes(pub Vec<u8>);

impl tauri::ipc::IpcResponse for TerminalBytes {
    fn body(self) -> tauri::Result<tauri::ipc::InvokeResponseBody> {
        Ok(tauri::ipc::InvokeResponseBody::Raw(self.0))
    }
}

/// An attached pane's channel.
pub struct ChannelSink(pub tauri::ipc::Channel<TerminalBytes>);

impl OutputSink for ChannelSink {
    fn send(&self, bytes: Vec<u8>) -> bool {
        self.0.send(TerminalBytes(bytes)).is_ok()
    }
}

/// The PATH a GUI app's children should get: the login shell's, else the app's own.
fn child_path() -> Option<String> {
    login_shell_path().or_else(|| std::env::var("PATH").ok())
}

/// `program` as an executable path: an absolute one must exist and be executable, a bare name is looked up on the
/// login shell's PATH.
fn resolve(program: &str, cwd: &Path) -> Result<PathBuf, Error> {
    let missing = || Error::Precondition(format!("{program} not found on PATH"));
    if Path::new(program).is_absolute() {
        return which::which(program).map_err(|_| missing());
    }
    which::which_in(program, child_path(), cwd).map_err(|_| missing())
}

fn pty_err(e: impl std::fmt::Display) -> Error {
    Error::Io(format!("Couldn't open a terminal: {e}"))
}

/// The reader's gate: closed while flow control has the session paused.
#[derive(Default)]
struct Gate {
    paused: Mutex<bool>,
    cv: Condvar,
}

impl Gate {
    fn set(&self, paused: bool) {
        *self.paused.lock().expect("gate lock") = paused;
        self.cv.notify_all();
    }

    fn wait_open(&self) {
        let mut paused = self.paused.lock().expect("gate lock");
        while *paused {
            paused = self.cv.wait(paused).expect("gate lock");
        }
    }
}

/// The open frame, shared by the reader, the flusher and the waiter. Frames go out while this lock is held, so
/// they reach `on_output` in order.
struct Frames {
    state: Mutex<FrameState>,
    cv: Condvar,
}

struct FrameState {
    batcher: Batcher,
    on_output: OutputFn,
    /// The reader hit EOF.
    eof: bool,
    /// The waiter has reported the exit; the flusher may stop.
    closed: bool,
}

impl FrameState {
    fn emit(&mut self, frame: Option<Vec<u8>>) {
        if let Some(f) = frame {
            (self.on_output)(&f);
        }
    }
}

pub struct PortablePtySpawner;

impl PtySpawner for PortablePtySpawner {
    fn spawn(&self, spec: &SpawnSpec, on_output: OutputFn, on_exit: ExitFn) -> Result<Box<dyn Pty>, Error> {
        let program = resolve(&spec.program, &spec.cwd)?;
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: spec.rows,
                cols: spec.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_err)?;
        let mut cmd = CommandBuilder::new(&program);
        cmd.args(&spec.args);
        cmd.cwd(&spec.cwd);
        if let Some(path) = child_path() {
            cmd.env("PATH", path);
        }
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        let mut child = pair.slave.spawn_command(cmd).map_err(pty_err)?;
        drop(pair.slave); // so the reader sees EOF once the child and its jobs are gone
        let reader = pair.master.try_clone_reader().map_err(pty_err)?;
        let writer = pair.master.take_writer().map_err(pty_err)?;
        let killer = child.clone_killer();
        let pid = child.process_id();

        let gate = Arc::new(Gate::default());
        let exit = Arc::new(Mutex::new(None));
        let frames = Arc::new(Frames {
            state: Mutex::new(FrameState {
                batcher: Batcher::new(Batcher::INTERVAL, Batcher::MAX_BYTES),
                on_output,
                eof: false,
                closed: false,
            }),
            cv: Condvar::new(),
        });
        spawn_reader(reader, gate.clone(), frames.clone());
        spawn_flusher(frames.clone());
        let (exit2, frames2) = (exit.clone(), frames);
        std::thread::Builder::new()
            .name("pty-wait".into())
            .spawn(move || {
                let code = child.wait().map(|s| s.exit_code() as i32).unwrap_or(-1);
                *exit2.lock().expect("exit lock") = Some(code);
                let mut st = frames2.state.lock().expect("frames lock");
                let deadline = Instant::now() + DRAIN_WAIT;
                while !st.eof && Instant::now() < deadline {
                    st = frames2
                        .cv
                        .wait_timeout(st, deadline - Instant::now())
                        .expect("frames lock")
                        .0;
                }
                let last = st.batcher.flush();
                st.emit(last);
                st.closed = true;
                frames2.cv.notify_all();
                drop(st);
                on_exit(code);
            })
            .map_err(|e| Error::Io(e.to_string()))?;
        Ok(Box::new(PortablePty {
            master: pair.master,
            writer,
            killer,
            pid,
            exit,
            gate,
        }))
    }
}

fn spawn_reader(mut reader: Box<dyn Read + Send>, gate: Arc<Gate>, frames: Arc<Frames>) {
    let _ = std::thread::Builder::new().name("pty-read".into()).spawn(move || {
        let mut buf = vec![0u8; READ_CHUNK];
        loop {
            gate.wait_open();
            let n = match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            let mut st = frames.state.lock().expect("frames lock");
            let full = st.batcher.push(&buf[..n], Instant::now());
            st.emit(full);
            frames.cv.notify_all();
        }
        let mut st = frames.state.lock().expect("frames lock");
        let last = st.batcher.flush();
        st.emit(last);
        st.eof = true;
        frames.cv.notify_all();
    });
}

fn spawn_flusher(frames: Arc<Frames>) {
    let _ = std::thread::Builder::new()
        .name("pty-flush".into())
        .spawn(move || loop {
            let mut st = frames.state.lock().expect("frames lock");
            while st.batcher.is_empty() && !st.eof && !st.closed {
                st = frames.cv.wait(st).expect("frames lock");
            }
            let Some(due) = st.batcher.due_at() else { return }; // empty after EOF or close: done
            let now = Instant::now();
            if due > now {
                st = frames.cv.wait_timeout(st, due - now).expect("frames lock").0;
            }
            let frame = st.batcher.flush_if_due(Instant::now());
            st.emit(frame);
        });
}

struct PortablePty {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    pid: Option<u32>,
    exit: Arc<Mutex<Option<i32>>>,
    gate: Arc<Gate>,
}

impl Pty for PortablePty {
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::Io(format!("Couldn't resize the terminal: {e}")))
    }

    /// SIGKILL: the polite end (exit keys, then the grace period) already happened in `stop::run`.
    fn kill(&mut self) -> Result<(), Error> {
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            // SAFETY: kill(2) with a pid we spawned and a constant signal has no memory effects.
            if unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) } == 0 {
                return Ok(());
            }
        }
        self.killer.kill().map_err(Error::from)
    }

    fn try_wait(&mut self) -> Result<Option<i32>, Error> {
        Ok(*self.exit.lock().expect("exit lock"))
    }

    fn pause_output(&self, paused: bool) {
        self.gate.set(paused);
    }
}

impl Drop for PortablePty {
    fn drop(&mut self) {
        self.gate.set(false); // a reader parked at the gate drains to EOF and ends
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;

    fn spec(cmd: &str) -> SpawnSpec {
        SpawnSpec {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), cmd.into()],
            cwd: std::env::temp_dir(),
            env: vec![],
            cols: 80,
            rows: 24,
        }
    }

    /// Spawns `cmd`, optionally types `input`, and gathers its output until it exits (5 s at most).
    fn collect_with_input(cmd: &str, input: &[u8]) -> (Vec<u8>, i32) {
        let out: Arc<Mutex<Vec<u8>>> = Arc::default();
        let sink = out.clone();
        let (tx, rx) = mpsc::channel();
        let mut pty = PortablePtySpawner
            .spawn(
                &spec(cmd),
                Box::new(move |b| sink.lock().unwrap().extend_from_slice(b)),
                Box::new(move |code| tx.send(code).unwrap()),
            )
            .unwrap();
        if !input.is_empty() {
            pty.write(input).unwrap();
        }
        let code = rx.recv_timeout(Duration::from_secs(5)).expect("exit within 5 s");
        let bytes = out.lock().unwrap().clone();
        (bytes, code)
    }

    fn collect(cmd: &str) -> (Vec<u8>, i32) {
        collect_with_input(cmd, b"")
    }

    #[test]
    fn streams_output_and_reports_exit_code() {
        let (out, code) = collect("printf hello; sleep 0.1; printf world; exit 3");
        assert!(String::from_utf8_lossy(&out).contains("helloworld"));
        assert_eq!(code, 3);
    }

    #[test]
    fn write_reaches_the_child() {
        let (out, _) = collect_with_input("read x; printf \"got:$x\"", b"abc\n");
        assert!(String::from_utf8_lossy(&out).contains("got:abc"));
    }

    #[test]
    fn spawn_unknown_program_is_precondition() {
        let s = SpawnSpec {
            program: "definitely-not-a-binary-xyz".into(),
            ..spec("")
        };
        assert!(matches!(
            PortablePtySpawner.spawn(&s, Box::new(|_| {}), Box::new(|_| {})),
            Err(Error::Precondition(m)) if m.contains("definitely-not-a-binary-xyz")
        ));
    }

    #[test]
    fn paused_gate_stops_reads_until_resumed() {
        let out: Arc<Mutex<Vec<u8>>> = Arc::default();
        let sink = out.clone();
        let (first_tx, first_rx) = mpsc::channel::<()>();
        let (tx, rx) = mpsc::channel();
        let pty = PortablePtySpawner
            .spawn(
                &spec("yes | head -c 5000000"),
                Box::new(move |b| {
                    sink.lock().unwrap().extend_from_slice(b);
                    let _ = first_tx.send(());
                }),
                Box::new(move |code| tx.send(code).unwrap()),
            )
            .unwrap();
        first_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        pty.pause_output(true);
        std::thread::sleep(Duration::from_millis(100)); // the read in flight lands
        let held = out.lock().unwrap().len();
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(out.lock().unwrap().len(), held, "no reads while paused");
        assert!(held < 7_500_000);
        pty.pause_output(false);
        rx.recv_timeout(Duration::from_secs(10)).expect("exit after resume");
        let bytes = out.lock().unwrap().clone();
        // the terminal turns each \n into \r\n, so count the payload: every y and every newline arrived
        assert_eq!(bytes.iter().filter(|b| **b == b'y').count(), 2_500_000);
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 2_500_000);
    }

    #[test]
    fn kill_ends_the_process() {
        let (tx, rx) = mpsc::channel();
        let mut pty = PortablePtySpawner
            .spawn(
                &spec("sleep 30"),
                Box::new(|_| {}),
                Box::new(move |code| tx.send(code).unwrap()),
            )
            .unwrap();
        pty.kill().unwrap();
        rx.recv_timeout(Duration::from_secs(2)).expect("exit within 2 s");
        assert!(pty.try_wait().unwrap().is_some());
    }
}
