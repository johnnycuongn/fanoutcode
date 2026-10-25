//! Task memory folders (§8): `<memory_root>/YYYY-MM-DD-<slug>/` with the stage files and `index.md`.

pub mod index;
pub mod slug;
pub mod store;

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};

use crate::domain::{MemoryLocation, Project};
use crate::{Error, Result};

pub use index::{IndexDoc, IndexEntry, IndexFrontMatter, StateWord, TimelineKind};
pub use store::Loaded;

/// The files of a task folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryFile {
    Intent,
    Spec,
    Plan,
    Review,
    Commits,
    Index,
}

impl MemoryFile {
    pub const ALL: [MemoryFile; 6] = [
        MemoryFile::Intent,
        MemoryFile::Spec,
        MemoryFile::Plan,
        MemoryFile::Review,
        MemoryFile::Commits,
        MemoryFile::Index,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            MemoryFile::Intent => "intent.md",
            MemoryFile::Spec => "spec.md",
            MemoryFile::Plan => "plan.md",
            MemoryFile::Review => "review.md",
            MemoryFile::Commits => "commits.md",
            MemoryFile::Index => "index.md",
        }
    }

    /// Accepts `"plan"` or `"plan.md"`.
    pub fn parse(name: &str) -> Option<MemoryFile> {
        let stem = name.strip_suffix(".md").unwrap_or(name);
        MemoryFile::ALL
            .into_iter()
            .find(|f| f.file_name().strip_suffix(".md") == Some(stem))
    }

    /// Only the app writes `index.md`.
    pub fn app_owned(self) -> bool {
        self == MemoryFile::Index
    }
}

/// Where a project's task folders live: `<app_data>/projects/<project-id>/tasks` for app storage, else the chosen
/// folder.
pub fn root_for(project: &Project, app_data: &Path) -> PathBuf {
    match &project.memory_location {
        MemoryLocation::AppStorage => app_data.join("projects").join(project.id.as_str()).join("tasks"),
        MemoryLocation::Folder { path } => path.clone(),
    }
}

/// One task's memory folder. Reads of a missing folder return nothing; writes to it fail; nothing recreates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFolder {
    dir: PathBuf,
}

impl TaskFolder {
    /// Makes `<root>/<unique folder name>` and writes `intent.md` verbatim.
    pub fn create(root: &Path, date: NaiveDate, title: &str, intent: &str) -> Result<TaskFolder> {
        std::fs::create_dir_all(root)?;
        let dir = root.join(slug::unique_folder_name(root, date, title));
        std::fs::create_dir(&dir)?;
        let folder = TaskFolder { dir };
        folder.write(MemoryFile::Intent, intent)?;
        Ok(folder)
    }

    pub fn open(dir: PathBuf) -> TaskFolder {
        TaskFolder { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn is_missing(&self) -> bool {
        !self.dir.is_dir()
    }

    pub fn path(&self, file: MemoryFile) -> PathBuf {
        self.dir.join(file.file_name())
    }

    /// `None` when the file or the folder is missing; a file that is not UTF-8 is set aside and reads as `None`.
    pub fn read(&self, file: MemoryFile) -> Result<Option<String>> {
        if self.is_missing() {
            return Ok(None);
        }
        Ok(match store::load_text(&self.path(file))? {
            Loaded::Parsed(text) => Some(text),
            Loaded::Missing | Loaded::Corrupt { .. } => None,
        })
    }

    /// Atomic. `index.md` is refused (the app writes it through `write_index`), and so is a missing folder.
    pub fn write(&self, file: MemoryFile, contents: &str) -> Result<()> {
        if file.app_owned() {
            return Err(Error::Invalid("index.md is written by the app".into()));
        }
        self.require_dir()?;
        store::write_atomic(&self.path(file), contents.as_bytes())
    }

    /// A corrupt `index.md` is set aside as `index.md.bad`, never overwritten.
    pub fn read_index(&self) -> Result<Loaded<IndexDoc>> {
        if self.is_missing() {
            return Ok(Loaded::Missing);
        }
        store::load_with(&self.path(MemoryFile::Index), index::parse)
    }

    pub fn write_index(&self, doc: &IndexDoc) -> Result<()> {
        self.require_dir()?;
        store::write_atomic(&self.path(MemoryFile::Index), index::render(doc).as_bytes())
    }

    /// The file was modified after `since`; `false` when it is missing.
    pub fn modified_since(&self, file: MemoryFile, since: DateTime<Utc>) -> Result<bool> {
        match std::fs::metadata(self.path(file)).and_then(|m| m.modified()) {
            Ok(modified) => Ok(DateTime::<Utc>::from(modified) > since),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    fn require_dir(&self) -> Result<()> {
        if self.is_missing() {
            return Err(Error::Precondition(format!(
                "Task memory folder is missing: {}",
                self.dir.display()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::domain::MemoryLocation;
    use crate::test_support::project_fixture;
    use crate::Error;

    #[test]
    fn create_writes_intent_verbatim_in_a_dated_folder() {
        let dir = tempfile::tempdir().unwrap();
        let d = chrono::NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        let f = TaskFolder::create(dir.path(), d, "Add dark mode", "# Add dark mode\n\nPlease.\n").unwrap();
        assert_eq!(f.dir(), dir.path().join("2026-10-24-add-dark-mode"));
        assert_eq!(
            f.read(MemoryFile::Intent).unwrap().as_deref(),
            Some("# Add dark mode\n\nPlease.\n")
        );
        assert_eq!(f.read(MemoryFile::Plan).unwrap(), None);
        assert!(!f.is_missing());
    }

    #[test]
    fn write_refuses_index_and_writes_the_rest_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let f = TaskFolder::open(dir.path().join("t"));
        std::fs::create_dir(f.dir()).unwrap();
        assert!(matches!(f.write(MemoryFile::Index, "x"), Err(Error::Invalid(_))));
        f.write(MemoryFile::Plan, "- [ ] 1. Do it\n").unwrap();
        assert!(!f.dir().join("plan.md.tmp").exists());
        assert_eq!(f.read(MemoryFile::Plan).unwrap().as_deref(), Some("- [ ] 1. Do it\n"));
    }

    #[test]
    fn missing_folder_reads_none_and_refuses_writes_without_recreating() {
        let dir = tempfile::tempdir().unwrap();
        let f = TaskFolder::open(dir.path().join("gone"));
        assert!(f.is_missing());
        assert_eq!(f.read(MemoryFile::Spec).unwrap(), None);
        assert!(matches!(f.write(MemoryFile::Spec, "s"), Err(Error::Precondition(_))));
        assert!(matches!(f.read_index().unwrap(), Loaded::Missing));
        assert!(!f.dir().exists(), "nothing recreated the folder");
    }

    #[test]
    fn corrupt_index_is_set_aside_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let f = TaskFolder::open(dir.path().join("t"));
        std::fs::create_dir(f.dir()).unwrap();
        std::fs::write(f.path(MemoryFile::Index), "not front matter").unwrap();
        assert!(matches!(f.read_index().unwrap(), Loaded::Corrupt { kept: Some(_) }));
        assert_eq!(
            std::fs::read_to_string(f.dir().join("index.md.bad")).unwrap(),
            "not front matter"
        );
    }

    #[test]
    fn modified_since_sees_a_later_write() {
        let dir = tempfile::tempdir().unwrap();
        let f = TaskFolder::open(dir.path().join("t"));
        std::fs::create_dir(f.dir()).unwrap();
        let before = chrono::Utc::now() - chrono::Duration::seconds(5);
        assert!(!f.modified_since(MemoryFile::Plan, before).unwrap());
        f.write(MemoryFile::Plan, "p").unwrap();
        assert!(f.modified_since(MemoryFile::Plan, before).unwrap());
        assert!(!f
            .modified_since(MemoryFile::Plan, chrono::Utc::now() + chrono::Duration::seconds(5))
            .unwrap());
    }

    #[test]
    fn root_for_follows_the_memory_location() {
        let p = project_fixture("p-1a-1", MemoryLocation::AppStorage);
        assert_eq!(
            root_for(&p, Path::new("/data")),
            PathBuf::from("/data/projects/p-1a-1/tasks")
        );
        let p = project_fixture(
            "p-1a-1",
            MemoryLocation::Folder {
                path: "/repo/docs/tasks".into(),
            },
        );
        assert_eq!(root_for(&p, Path::new("/data")), PathBuf::from("/repo/docs/tasks"));
    }
}
