//! Stage helpers. `as_str` / `parse` are the lower-case words used in `index.md` and sentinels only; serde and
//! SQLite use the Rust names `"Spec"` … `"Done"` (A16).

use crate::domain::Stage;

/// What a stage must produce before its sentinel counts as done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageArtifact {
    PlanMd,
    ChangedFiles,
    ReviewMd,
}

impl Stage {
    pub const ALL: [Stage; 7] = [
        Stage::Spec,
        Stage::Plan,
        Stage::Build,
        Stage::Review,
        Stage::Test,
        Stage::Commit,
        Stage::Done,
    ];

    /// The column position, 0 (Spec) to 6 (Done).
    pub fn index(self) -> usize {
        Stage::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every stage is in ALL")
    }

    /// The following column; `None` after Done.
    pub fn next(self) -> Option<Stage> {
        Stage::ALL.get(self.index() + 1).copied()
    }

    /// The line an agent ends its final message with: `TEAMCODE: plan done`.
    pub fn sentinel(self) -> String {
        format!("{}{} done", super::handback::SENTINEL_PREFIX, self.as_str())
    }

    pub fn artifact(self) -> Option<StageArtifact> {
        match self {
            Stage::Plan => Some(StageArtifact::PlanMd),
            Stage::Build => Some(StageArtifact::ChangedFiles),
            Stage::Review => Some(StageArtifact::ReviewMd),
            _ => None,
        }
    }

    /// Spec, Plan, Build, Review, Test (the fix prompt) and Commit (the draft prompt) send a prompt; Done does not.
    pub fn has_agent_prompt(self) -> bool {
        self != Stage::Done
    }

    /// `"spec"` … `"done"`: `index.md` lines and sentinels only.
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Spec => "spec",
            Stage::Plan => "plan",
            Stage::Build => "build",
            Stage::Review => "review",
            Stage::Test => "test",
            Stage::Commit => "commit",
            Stage::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Option<Stage> {
        Some(match s {
            "spec" => Stage::Spec,
            "plan" => Stage::Plan,
            "build" => Stage::Build,
            "review" => Stage::Review,
            "test" => Stage::Test,
            "commit" => Stage::Commit,
            "done" => Stage::Done,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_strings_round_trip_and_order_holds() {
        for s in Stage::ALL {
            assert_eq!(Stage::parse(s.as_str()), Some(s));
        }
        assert_eq!(Stage::Spec.next(), Some(Stage::Plan));
        assert_eq!(Stage::Done.next(), None);
        assert!(Stage::Build.index() < Stage::Review.index());
        assert_eq!(Stage::Plan.sentinel(), "TEAMCODE: plan done");
        assert_eq!(Stage::Build.artifact(), Some(StageArtifact::ChangedFiles));
        assert_eq!(Stage::Test.artifact(), None);
    }
}
