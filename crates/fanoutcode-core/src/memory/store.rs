//! File helpers for memory folders and other user-visible files: atomic writes, and setting aside a file that does
//! not parse instead of overwriting it, because it may hold the user's work. Modelled on LiveSpec's `store.rs`.

use std::path::{Path, PathBuf};

use crate::Result;

/// What a load found.
#[derive(Debug)]
pub enum Loaded<T> {
    /// No file.
    Missing,
    Parsed(T),
    /// The file did not parse. It was renamed to `kept`; `None` if even that failed.
    Corrupt {
        kept: Option<PathBuf>,
    },
}

/// Atomic write: `<file>.tmp` beside the target, then rename over it; creates the parent folders.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = with_suffix(path, ".tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Renames a corrupt file to the first free `<name>.bad`, `<name>.bad.1`, …; never replaces an earlier copy.
pub fn set_aside(path: &Path) -> Option<PathBuf> {
    let kept = (0u32..)
        .map(|n| with_suffix(path, &if n == 0 { ".bad".to_owned() } else { format!(".bad.{n}") }))
        .find(|p| !p.exists())?;
    std::fs::rename(path, &kept).ok()?;
    Some(kept)
}

/// `Missing` on NotFound, `Parsed` on valid UTF-8, `Corrupt` (file set aside) otherwise. Any other read error is
/// `Err` and the file is left in place.
pub fn load_text(path: &Path) -> Result<Loaded<String>> {
    load_with(path, |s| Some(s.to_owned()))
}

/// As `load_text`, then `parse`; `None` from `parse` means corrupt and the file is set aside.
pub fn load_with<T>(path: &Path, parse: impl FnOnce(&str) -> Option<T>) -> Result<Loaded<T>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(match std::str::from_utf8(&bytes).ok().and_then(parse) {
            Some(value) => Loaded::Parsed(value),
            None => Loaded::Corrupt { kept: set_aside(path) },
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Loaded::Missing),
        Err(e) => Err(e.into()),
    }
}

/// `plan.md` + `.tmp` = `plan.md.tmp`.
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_creates_parents_and_leaves_no_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/plan.md");
        write_atomic(&path, b"one").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "one");
        assert!(!dir.path().join("a/b/plan.md.tmp").exists());
        write_atomic(&path, b"two").unwrap();
        assert!(matches!(load_text(&path).unwrap(), Loaded::Parsed(s) if s == "two"));
    }

    #[test]
    fn missing_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            load_text(&dir.path().join("none.md")).unwrap(),
            Loaded::Missing
        ));
    }

    #[test]
    fn corrupt_file_is_set_aside_and_earlier_copies_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.md");
        for (i, body) in ["first", "second", "third"].iter().enumerate() {
            std::fs::write(&path, body).unwrap();
            let Loaded::Corrupt { kept: Some(kept) } = load_with(&path, |_| None::<()>).unwrap() else {
                panic!("not kept")
            };
            let name = if i == 0 {
                "index.md.bad".to_owned()
            } else {
                format!("index.md.bad.{i}")
            };
            assert_eq!(kept, dir.path().join(name));
            assert!(!path.exists());
        }
        assert_eq!(
            std::fs::read_to_string(dir.path().join("index.md.bad")).unwrap(),
            "first"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("index.md.bad.2")).unwrap(),
            "third"
        );
    }

    #[test]
    fn unreadable_path_is_an_error_not_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.md");
        std::fs::create_dir(&path).unwrap();
        assert!(load_text(&path).is_err());
        assert!(path.is_dir());
        assert!(!dir.path().join("index.md.bad").exists());
    }
}
