//! The PATH the app's children get: sessions, the test command, the agent check and the Claude CLI provider.
//!
//! An app opened from Finder starts with launchd's bare PATH (`/usr/bin:/bin:/usr/sbin:/sbin`), so `claude`, `codex`
//! and `node` from the Claude Code installer, Homebrew, npm or nvm are missing. The login shell knows the real PATH,
//! but an rc file that loads nvm can take 15 s or more to answer. So the shell is asked once, in the background, from
//! launch; a lookup waits for it at most `SHELL_WAIT` after launch, and until it answers (or when it fails) the
//! stand-in is used: the usual install folders that exist, then the app's own PATH. Every lookup after the shell
//! answers gets the shell's PATH.
//!
//! Adapts `livespec_core::shell_env`, which gives up on the shell after 4 s and keeps the bare PATH for good.

use std::collections::HashSet;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// How long after launch a lookup waits for the login shell before using the stand-in.
pub const SHELL_WAIT: Duration = Duration::from_secs(4);
/// When the login shell is given up on and killed (an rc file waiting for something that never comes).
const SHELL_GIVE_UP: Duration = Duration::from_secs(120);
/// Brackets the PATH in the shell's output, so whatever the rc files print is ignored.
const MARK: &str = "__FANOUTCODE_PATH__";

static PROBE: OnceLock<Arc<Probe>> = OnceLock::new();

/// Starts asking the login shell; call it at launch. Later calls do nothing.
pub fn start() {
    probe();
}

/// The PATH for a child: the login shell's once it has answered, else the stand-in. Blocking (up to `SHELL_WAIT`
/// after `start`), so call it from a blocking context.
pub fn path() -> String {
    probe()
        .answer(SHELL_WAIT)
        .unwrap_or_else(|| stand_in(std::env::var("PATH").ok().as_deref(), home_dir().as_deref()))
}

fn probe() -> &'static Arc<Probe> {
    PROBE.get_or_init(|| Probe::spawn(read_login_shell_path))
}

/// One question to the login shell, answered on its own thread.
struct Probe {
    started: Instant,
    /// `None` while the shell runs; then its PATH, or `None` when it failed.
    answer: Mutex<Option<Option<String>>>,
    answered: Condvar,
}

impl Probe {
    fn spawn(ask: impl FnOnce() -> Option<String> + Send + 'static) -> Arc<Probe> {
        let probe = Arc::new(Probe {
            started: Instant::now(),
            answer: Mutex::new(None),
            answered: Condvar::new(),
        });
        let p = probe.clone();
        std::thread::spawn(move || {
            let path = ask();
            *p.answer.lock().expect("shell path lock") = Some(path);
            p.answered.notify_all();
        });
        probe
    }

    /// The shell's PATH, waited for until `wait` after the probe started; `None` when the shell failed, or is still
    /// running after that.
    fn answer(&self, wait: Duration) -> Option<String> {
        let deadline = self.started + wait;
        let mut answer = self.answer.lock().expect("shell path lock");
        loop {
            if let Some(path) = answer.as_ref() {
                return path.clone();
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return None;
            }
            answer = self.answered.wait_timeout(answer, left).expect("shell path lock").0;
        }
    }
}

/// `$SHELL -ilc` printing its PATH between markers. The output is read only up to the closing marker (a daemon an rc
/// file starts may hold the pipe open), and the shell is killed after `SHELL_GIVE_UP`.
fn read_login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let mut child = Command::new(shell)
        .args(["-ilc", &format!("printf '{MARK}%s{MARK}' \"$PATH\"")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(read_marked(stdout));
    });
    let path = rx.recv_timeout(SHELL_GIVE_UP).ok().flatten();
    // the PATH is in (or never coming): the shell has nothing left to say
    let _ = child.kill();
    let _ = child.wait();
    path
}

/// Reads `out` until the PATH between the markers is all there; `None` when the output ends without it.
fn read_marked(mut out: impl Read) -> Option<String> {
    let (mut seen, mut chunk) = (Vec::new(), [0u8; 4096]);
    loop {
        match out.read(&mut chunk) {
            Ok(0) => return None,
            Ok(n) => seen.extend_from_slice(&chunk[..n]),
            // a signal to the app (one of its own children exiting) interrupts the read; the shell is still talking
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
        if let Some(path) = marked(&String::from_utf8_lossy(&seen)) {
            return Some(path);
        }
    }
}

/// The PATH between the two markers in the shell's output, when it is all there and not empty.
fn marked(out: &str) -> Option<String> {
    let start = out.find(MARK)? + MARK.len();
    let end = start + out[start..].find(MARK)?;
    Some(out[start..end].to_owned()).filter(|p| !p.is_empty())
}

/// The PATH used until the login shell answers: nvm's Node and the usual install folders that exist, then
/// `app_path`, each folder once.
fn stand_in(app_path: Option<&str>, home: Option<&Path>) -> String {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(home) = home {
        dirs.extend(nvm_node(home));
        for rel in [".local/bin", ".claude/local", ".npm-global/bin", ".bun/bin", ".volta/bin", ".cargo/bin"] {
            dirs.push(home.join(rel));
        }
    }
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    dirs.retain(|d| d.is_dir());
    dirs.extend(app_path.into_iter().flat_map(std::env::split_paths));
    let mut seen = HashSet::new();
    dirs.iter()
        .filter_map(|d| d.to_str())
        .filter(|d| !d.is_empty() && !d.contains(':') && seen.insert(*d))
        .collect::<Vec<_>>()
        .join(":")
}

/// nvm's Node `bin` folder (the npm build of Claude Code runs on `node`): the newest installed version matching
/// nvm's default alias when that is a version number, else the newest installed.
fn nvm_node(home: &Path) -> Option<PathBuf> {
    let nvm = home.join(".nvm");
    let default = std::fs::read_to_string(nvm.join("alias/default")).unwrap_or_default();
    let wanted = version(default.trim()).unwrap_or_default();
    std::fs::read_dir(nvm.join("versions/node"))
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            Some((version(entry.file_name().to_str()?)?, entry.path().join("bin")))
        })
        .filter(|(v, _)| v.starts_with(&wanted))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, bin)| bin)
}

/// `v24.15.0`, `24.15` or `24` as numbers; `None` for an alias such as `node` or `lts/*`.
fn version(name: &str) -> Option<Vec<u64>> {
    name.strip_prefix('v')
        .unwrap_or(name)
        .split('.')
        .map(|n| n.parse().ok())
        .collect()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkdirs(home: &Path, rels: &[&str]) {
        for rel in rels {
            std::fs::create_dir_all(home.join(rel)).unwrap();
        }
    }

    fn at(home: &Path, rel: &str) -> String {
        home.join(rel).display().to_string()
    }

    #[test]
    fn the_stand_in_puts_existing_install_folders_before_the_app_path_once() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        mkdirs(
            h,
            &[
                ".local/bin",
                ".nvm/versions/node/v9.0.0/bin",
                ".nvm/versions/node/v20.11.1/bin",
                ".nvm/versions/node/v24.15.0/bin",
            ],
        );
        let local = at(h, ".local/bin");
        let path = stand_in(Some(&format!("/usr/bin:{local}::/bin:/usr/bin")), Some(h));
        let dirs: Vec<&str> = path.split(':').collect();
        // nvm's newest Node (by number, not by name), then the Claude Code installer's folder
        assert_eq!(dirs[..2], [at(h, ".nvm/versions/node/v24.15.0/bin"), local.clone()]);
        // folders that don't exist are left out; the app's own PATH comes last, each folder once
        assert!(!path.contains(".bun") && !path.contains("v20.11.1"), "{path}");
        assert_eq!(dirs[dirs.len() - 2..], ["/usr/bin", "/bin"]);
        assert_eq!(dirs.iter().filter(|d| **d == local).count(), 1);
        assert!(!dirs.contains(&""));
    }

    #[test]
    fn the_stand_in_follows_nvm_s_default_version() {
        let home = tempfile::tempdir().unwrap();
        let h = home.path();
        mkdirs(
            h,
            &[
                ".nvm/alias",
                ".nvm/versions/node/v20.11.1/bin",
                ".nvm/versions/node/v22.9.0/bin",
                ".nvm/versions/node/v22.12.0/bin",
                ".nvm/versions/node/v24.15.0/bin",
            ],
        );
        std::fs::write(h.join(".nvm/alias/default"), "22\n").unwrap();
        let path = stand_in(None, Some(h));
        assert_eq!(path.split(':').next(), Some(at(h, ".nvm/versions/node/v22.12.0/bin").as_str()));

        // an alias that names no version picks the newest
        std::fs::write(h.join(".nvm/alias/default"), "lts/*\n").unwrap();
        let path = stand_in(None, Some(h));
        assert_eq!(path.split(':').next(), Some(at(h, ".nvm/versions/node/v24.15.0/bin").as_str()));
    }

    #[test]
    fn a_quick_shell_is_waited_for() {
        let probe = Probe::spawn(|| Some("/from/shell".into()));
        assert_eq!(probe.answer(Duration::from_secs(5)).as_deref(), Some("/from/shell"));
    }

    #[test]
    fn a_slow_shell_gives_way_to_the_stand_in_then_answers_later_lookups() {
        let (release, released) = mpsc::channel::<()>();
        let probe = Probe::spawn(move || {
            let _ = released.recv();
            Some("/from/shell".into())
        });
        let t = Instant::now();
        assert_eq!(probe.answer(Duration::from_millis(50)), None);
        assert!(t.elapsed() < Duration::from_secs(2));

        release.send(()).unwrap();
        // past the wait, a lookup no longer waits, but gets the shell's PATH once it is in
        let later = (0..500).find_map(|_| {
            std::thread::sleep(Duration::from_millis(10));
            probe.answer(Duration::ZERO)
        });
        assert_eq!(later.as_deref(), Some("/from/shell"));
    }

    #[test]
    fn a_failed_shell_answers_at_once() {
        let probe = Probe::spawn(|| None);
        let t = Instant::now();
        assert_eq!(probe.answer(Duration::from_secs(5)), None);
        assert!(t.elapsed() < Duration::from_secs(4));
    }

    #[test]
    fn the_path_is_read_between_the_markers() {
        assert_eq!(marked(&format!("Welcome!\n{MARK}/a:/b{MARK}\nbye")).as_deref(), Some("/a:/b"));
        assert_eq!(marked(&format!("{MARK}/a:/b")), None); // still being printed
        assert_eq!(marked(&format!("{MARK}{MARK}")), None);
    }

    /// Output that arrives in pieces, some reads failing.
    struct Pieces(std::collections::VecDeque<std::io::Result<String>>);

    impl Read for Pieces {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self.0.pop_front() {
                None => Ok(0),
                Some(piece) => {
                    let piece = piece?;
                    buf[..piece.len()].copy_from_slice(piece.as_bytes());
                    Ok(piece.len())
                }
            }
        }
    }

    #[test]
    fn a_read_interrupted_by_a_signal_keeps_reading() {
        // in the app, its own children exiting interrupt the read; the shell is still running its rc files
        let out = Pieces(
            [
                Ok("rc noise\n".to_owned()),
                Err(ErrorKind::Interrupted.into()),
                Ok(format!("{MARK}/a:/b")),
                Err(ErrorKind::Interrupted.into()),
                Ok(MARK.to_owned()),
            ]
            .into(),
        );
        assert_eq!(read_marked(out).as_deref(), Some("/a:/b"));
    }

    #[test]
    fn output_that_ends_or_fails_without_the_path_reads_as_none() {
        assert_eq!(read_marked(Pieces([Ok(format!("{MARK}/a:/b"))].into())), None);
        assert_eq!(read_marked(Pieces([Err(ErrorKind::BrokenPipe.into())].into())), None);
    }

    /// The case this module exists for. Build the tests, then run the binary with Finder's environment:
    /// `env -i HOME=$HOME SHELL=$SHELL PATH=/usr/bin:/bin:/usr/sbin:/sbin <test binary> --ignored shell_path`
    #[test]
    #[ignore = "needs Claude Code installed; run it under Finder's bare PATH"]
    fn claude_is_found_from_finder_without_waiting_for_a_slow_shell() {
        start();
        let t = Instant::now();
        let path = path();
        assert!(t.elapsed() < SHELL_WAIT + Duration::from_secs(1));
        assert!(crate::agent_check::find_on(&path, "claude").is_some(), "{path}");

        // however long the rc files take, the shell's own PATH arrives for later lookups
        let shell = probe().answer(SHELL_GIVE_UP).expect("the login shell's PATH");
        assert!(crate::agent_check::find_on(&shell, "claude").is_some(), "{shell}");
    }
}
