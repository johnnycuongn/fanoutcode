//! Headless helpers (§7.5). This plan defines only `HelperKind`, the payload of `Effect::RunHelper`; Plan 05 extends
//! this file with the runners.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum HelperKind {
    /// Names a task from its intent.
    Naming,
    /// Keeps `spec.md` in step with the change during Review.
    SpecWatcher,
    /// Drafts `commits.md` on entering Commit.
    CommitMessages,
    /// Writes a project's brief for linked-project prompts.
    ProjectBrief,
    /// Commits the drafted messages per repo (the Commit button, only when "Team Code may commit" is on).
    GitCommit,
}
