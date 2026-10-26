//! The owned test runner (§6.5): the project's command (and any linked projects' commands added to the task) as a
//! child the app owns, its output streamed and replayable to every attached caller, its exit code captured.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{mpsc, watch};

use crate::domain::{Project, Task, TaskId};
use crate::pipeline::TestRun;
use crate::{Error, Result};

/// Output lines kept for the result and the Fix failing tests prompt.
pub const TAIL_LINES: usize = 200;
/// Bytes of output a late attacher is replayed.
pub const REPLAY_CAP: usize = 4 * 1024 * 1024;

/// One command and the folder it runs in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TestCommand {
    pub cwd: PathBuf,
    pub command: String,
}

/// A finished run: the commands that ran (up to the first failure), its exit code and the last output lines.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TestResult {
    pub commands: Vec<TestCommand>,
    pub exit_code: i32,
    pub tail: Vec<String>,
}

impl TestResult {
    /// The pipeline's view of the run (`TransitionCtx.last_test`, the Test prompt's placeholders).
    pub fn as_test_run(&self) -> TestRun {
        TestRun {
            command: self
                .commands
                .iter()
                .map(|c| c.command.as_str())
                .collect::<Vec<_>>()
                .join(" ; "),
            exit_code: self.exit_code,
            tail: self.tail.join("\n"),
        }
    }
}

/// Splits streamed bytes into lines, keeping the last `TAIL_LINES` and the unfinished one.
#[derive(Default)]
struct Tail {
    lines: VecDeque<String>,
    partial: Vec<u8>,
}

impl Tail {
    fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            if b == b'\n' {
                let line = String::from_utf8_lossy(&self.partial).trim_end_matches('\r').to_owned();
                self.partial.clear();
                self.lines.push_back(line);
                if self.lines.len() > TAIL_LINES {
                    self.lines.pop_front();
                }
            } else {
                self.partial.push(b);
            }
        }
    }

    fn finish(mut self) -> Vec<String> {
        if !self.partial.is_empty() {
            self.feed(b"\n");
        }
        self.lines.into()
    }
}

/// Runs each command as `sh -c` in its folder, stderr merged into stdout, a `$ <command>  (<cwd>)` header sent
/// before each; stops at the first non-zero exit. Dropping the future kills the child. `path_env` is the login
/// shell's `PATH` (as sessions get), since an app opened from the Finder has only the system one and no `npm`.
pub async fn run(
    commands: &[TestCommand],
    path_env: Option<&str>,
    on_output: &mut (dyn FnMut(Vec<u8>) + Send),
) -> Result<TestResult> {
    let mut tail = Tail::default();
    let (mut ran, mut exit_code) = (Vec::new(), 0);
    for c in commands {
        on_output(format!("$ {}  ({})\n", c.command, c.cwd.display()).into_bytes());
        ran.push(c.clone());
        let mut cmd = Command::new("sh");
        if let Some(path) = path_env {
            cmd.env("PATH", path);
        }
        let mut child = cmd
            .args(["-c", &format!("exec 2>&1; {}", c.command)])
            .current_dir(&c.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::Io(format!("Couldn't run `{}`: {e}", c.command)))?;
        let mut stdout = child.stdout.take().expect("piped");
        let mut chunk = vec![0u8; 8 * 1024];
        loop {
            let n = stdout.read(&mut chunk).await?;
            if n == 0 {
                break;
            }
            tail.feed(&chunk[..n]);
            on_output(chunk[..n].to_vec());
        }
        exit_code = child.wait().await?.code().unwrap_or(-1);
        if exit_code != 0 {
            break;
        }
    }
    Ok(TestResult {
        commands: ran,
        exit_code,
        tail: tail.finish(),
    })
}

/// The prompt Fix failing tests sends (§6.5): every command run, the exit code and the last 200 lines.
pub fn fix_prompt(r: &TestResult) -> String {
    let mut p = String::from("Tests failed. Fix them, then run the same command until it passes.\n\n");
    for c in &r.commands {
        p.push_str(&format!("Command: {}  (in {})\n", c.command, c.cwd.display()));
    }
    p.push_str(&format!(
        "Exit code: {}\n\nLast {TAIL_LINES} lines of output:\n```\n{}\n```\n",
        r.exit_code,
        r.tail.join("\n")
    ));
    p
}

/// What a caller attached to a run receives: everything printed so far, then each new chunk, then the exit code.
pub struct Attached {
    pub replay: Vec<u8>,
    pub live: mpsc::UnboundedReceiver<Vec<u8>>,
    pub done: watch::Receiver<Option<i32>>,
}

/// One run's output, shared by every caller. The buffer and the subscriber list sit behind one lock, so nothing
/// printed between a caller's replay and its first live chunk is lost or doubled.
pub struct LiveOutput {
    inner: Mutex<Buffer>,
    done: watch::Sender<Option<i32>>,
}

/// The replay bytes and the attached callers' senders.
#[derive(Default)]
struct Buffer {
    bytes: Vec<u8>,
    subs: Vec<mpsc::UnboundedSender<Vec<u8>>>,
}

impl LiveOutput {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::default(),
            done: watch::channel(None).0,
        })
    }

    /// Appends (keeping the last `REPLAY_CAP` bytes) and sends to every attached caller still listening.
    pub fn push(&self, bytes: &[u8]) {
        let mut inner = self.inner.lock().expect("live output lock");
        let Buffer { bytes: buf, subs } = &mut *inner;
        buf.extend_from_slice(bytes);
        if buf.len() > REPLAY_CAP {
            let extra = buf.len() - REPLAY_CAP;
            buf.drain(..extra);
        }
        subs.retain(|s| s.send(bytes.to_vec()).is_ok());
    }

    /// The run ended with `code`; attached callers see `done` become `Some(code)`.
    pub fn finish(&self, code: i32) {
        self.done.send_replace(Some(code));
    }

    pub fn attach(&self) -> Attached {
        let mut inner = self.inner.lock().expect("live output lock");
        let (tx, rx) = mpsc::unbounded_channel();
        inner.subs.push(tx);
        Attached {
            replay: inner.bytes.clone(),
            live: rx,
            done: self.done.subscribe(),
        }
    }
}

/// A linked project's test command for the task (§6.5), run in the project's root. Only a project linked to the
/// task, and only with a command.
pub fn linked_test_command(task: &Task, project: &Project, command: Option<&str>) -> Result<TestCommand> {
    if !task.linked_roots.contains(&project.root) {
        return Err(Error::Precondition("That project is not linked to this task.".into()));
    }
    match command.map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => Ok(TestCommand {
            cwd: project.root.clone(),
            command: c.to_owned(),
        }),
        None => Err(Error::Precondition(
            "That project has no test command. Set one in its project settings.".into(),
        )),
    }
}

/// Adds (or replaces) the task's command for that folder.
pub fn save_command(conn: &Connection, task_id: &TaskId, c: &TestCommand) -> Result<()> {
    conn.execute(
        "INSERT INTO task_test_commands (task_id, repo_root, command) VALUES (?1, ?2, ?3)
         ON CONFLICT (task_id, repo_root) DO UPDATE SET command = excluded.command",
        params![task_id.as_str(), c.cwd.to_string_lossy(), c.command],
    )?;
    Ok(())
}

/// The linked projects' commands added to the task, in folder order.
pub fn commands_for(conn: &Connection, task_id: &TaskId) -> Result<Vec<TestCommand>> {
    let mut stmt =
        conn.prepare("SELECT repo_root, command FROM task_test_commands WHERE task_id = ?1 ORDER BY repo_root")?;
    let rows = stmt.query_map([task_id.as_str()], |r| {
        Ok(TestCommand {
            cwd: PathBuf::from(r.get::<_, String>(0)?),
            command: r.get(1)?,
        })
    })?;
    rows.map(|r| r.map_err(Error::from)).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::db;
    use crate::domain::{MemoryLocation, Project};
    use crate::test_support::{project_fixture, task_fixture};

    fn cmd(command: &str) -> TestCommand {
        TestCommand {
            cwd: std::env::temp_dir(),
            command: command.into(),
        }
    }

    fn task_linked_to(roots: &[&str]) -> Task {
        let mut t = task_fixture("t-1a-1", "p-1a-1");
        t.linked_roots = roots.iter().map(PathBuf::from).collect();
        t
    }

    fn project_at(id: &str, root: &str) -> Project {
        let mut p = project_fixture(id, MemoryLocation::AppStorage);
        p.root = root.into();
        p
    }

    #[tokio::test]
    async fn the_given_path_is_the_commands_path() {
        let r = run(
            &[cmd("echo \"$PATH\"")],
            Some("/opt/login/bin:/usr/bin:/bin"),
            &mut |_| {},
        )
        .await
        .unwrap();
        assert!(
            r.tail.iter().any(|l| l == "/opt/login/bin:/usr/bin:/bin"),
            "{:?}",
            r.tail
        );
    }

    #[tokio::test]
    async fn run_streams_output_and_returns_the_exit_code() {
        let mut out = Vec::new();
        let r = run(&[cmd("printf 'a\\nb\\n'; echo err 1>&2; exit 3")], None, &mut |b| {
            out.extend(b)
        })
        .await
        .unwrap();
        assert_eq!(r.exit_code, 3);
        assert_eq!(r.tail, vec!["a", "b", "err"]);
        assert!(String::from_utf8(out).unwrap().starts_with("$ printf"));
    }

    #[tokio::test]
    async fn tail_keeps_the_last_200_lines() {
        let r = run(&[cmd("seq 1 250")], None, &mut |_| {}).await.unwrap();
        assert_eq!(
            (r.tail.len(), r.tail[0].as_str(), r.tail[199].as_str(), r.exit_code),
            (200, "51", "250", 0)
        );
    }

    #[tokio::test]
    async fn a_failing_command_stops_the_sequence() {
        let r = run(&[cmd("exit 2"), cmd("echo never")], None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!((r.exit_code, r.commands.len()), (2, 1));
        assert!(r.tail.is_empty());
    }

    #[test]
    fn fix_prompt_carries_command_code_and_tail() {
        let r = TestResult {
            commands: vec![cmd("npm test")],
            exit_code: 1,
            tail: vec!["FAIL x".into()],
        };
        let p = fix_prompt(&r);
        assert!(p.starts_with(
            "Tests failed. Fix them, then run the same command until it passes.\n\nCommand: npm test  (in "
        ));
        assert!(p.contains("\nExit code: 1\n\nLast 200 lines of output:\n```\nFAIL x\n```"));
    }

    #[tokio::test]
    async fn an_attach_mid_run_replays_what_was_printed_then_streams_the_rest() {
        let out = LiveOutput::new();
        out.push(b"$ npm test\nok 1\n");
        let mut a = out.attach();
        assert_eq!(a.replay, b"$ npm test\nok 1\n");
        out.push(b"ok 2\n");
        out.finish(0);
        assert_eq!(a.live.recv().await.unwrap(), b"ok 2\n");
        a.done.wait_for(|c| c.is_some()).await.unwrap();
        assert_eq!(*a.done.borrow(), Some(0));
    }

    #[tokio::test]
    async fn two_attached_callers_both_receive_live_output() {
        let out = LiveOutput::new();
        let (mut a, mut b) = (out.attach(), out.attach());
        out.push(b"x");
        assert_eq!(
            (a.live.recv().await.unwrap(), b.live.recv().await.unwrap()),
            (b"x".to_vec(), b"x".to_vec())
        );
    }

    #[test]
    fn replay_keeps_the_last_bytes_within_the_cap() {
        let out = LiveOutput::new();
        out.push(&vec![b'a'; REPLAY_CAP]);
        out.push(b"tail");
        let r = out.attach().replay;
        assert_eq!(r.len(), REPLAY_CAP);
        assert!(r.ends_with(b"tail"));
    }

    #[test]
    fn a_linked_project_adds_its_command_in_its_root() {
        let (task, api) = (task_linked_to(&["/ws/api"]), project_at("p-2", "/ws/api"));
        assert_eq!(
            linked_test_command(&task, &api, Some("cargo test")).unwrap(),
            TestCommand {
                cwd: "/ws/api".into(),
                command: "cargo test".into()
            }
        );
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        save_command(
            &conn,
            &task.id,
            &linked_test_command(&task, &api, Some("cargo test")).unwrap(),
        )
        .unwrap();
        assert_eq!(commands_for(&conn, &task.id).unwrap().len(), 1);
    }

    #[test]
    fn an_unlinked_project_is_refused() {
        let r = linked_test_command(&task_linked_to(&[]), &project_at("p-2", "/ws/api"), Some("cargo test"));
        assert!(matches!(r, Err(Error::Precondition(m)) if m == "That project is not linked to this task."));
    }

    #[test]
    fn a_project_without_a_test_command_is_refused() {
        let r = linked_test_command(&task_linked_to(&["/ws/api"]), &project_at("p-2", "/ws/api"), None);
        assert!(
            matches!(r, Err(Error::Precondition(m)) if m == "That project has no test command. Set one in its project settings.")
        );
    }
}
