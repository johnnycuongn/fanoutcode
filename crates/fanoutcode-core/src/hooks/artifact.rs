//! Whether a stage's artifact changed since the stage was entered (§6): the second half of a stage hand-back.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::domain::Stage;

/// `name` in `memory_dir` was modified after `since`; `false` when missing.
fn written_since(memory_dir: &Path, name: &str, since: DateTime<Utc>) -> bool {
    std::fs::metadata(memory_dir.join(name))
        .and_then(|m| m.modified())
        .is_ok_and(|t| DateTime::<Utc>::from(t) > since)
}

/// Spec: `spec.md` written since entry; Plan: `plan.md`; Build: the task's files changed (`files_changed`);
/// Review: `review.md` written; Test, Commit and Done have no artifact.
pub fn artifact_changed(memory_dir: &Path, stage: Stage, entered_at: DateTime<Utc>, files_changed: bool) -> bool {
    match stage {
        Stage::Spec => written_since(memory_dir, "spec.md", entered_at),
        Stage::Plan => written_since(memory_dir, "plan.md", entered_at),
        Stage::Build => files_changed,
        Stage::Review => written_since(memory_dir, "review.md", entered_at),
        Stage::Test | Stage::Commit | Stage::Done => true,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::*;

    #[test]
    fn artifact_changed_per_stage() {
        let dir = tempfile::tempdir().unwrap();
        let entered = Utc::now() - Duration::seconds(60);
        assert!(!artifact_changed(dir.path(), Stage::Plan, entered, false));
        std::fs::write(dir.path().join("plan.md"), "- [ ] 1. x").unwrap();
        assert!(artifact_changed(dir.path(), Stage::Plan, entered, false));
        assert!(!artifact_changed(dir.path(), Stage::Build, entered, false));
        assert!(artifact_changed(dir.path(), Stage::Build, entered, true));
        assert!(!artifact_changed(dir.path(), Stage::Review, entered, false));
        std::fs::write(dir.path().join("review.md"), "src/a.rs:1 · high · x").unwrap();
        assert!(artifact_changed(dir.path(), Stage::Review, entered, false));
    }
}
