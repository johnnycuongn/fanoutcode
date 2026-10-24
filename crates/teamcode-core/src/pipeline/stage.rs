//! Stage helpers. `as_str` / `parse` are the lower-case words used in `index.md` and sentinels only; serde and
//! SQLite use the Rust names `"Spec"` … `"Done"` (A16).

use crate::domain::Stage;

impl Stage {
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
