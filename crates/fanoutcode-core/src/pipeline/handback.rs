//! Hand-back detection (§6) and the chip text shown for a hand-back (A2). `chip_text` is the source of truth; Plan
//! 06's `handbackLabel(task)` in `src/lib/format.ts` mirrors it and is tested against `HANDBACK_FIXTURE`.

#[cfg(test)]
use std::path::Path;

use crate::domain::{HandbackReason, Stage};

/// The start of every sentinel line; the full line is `FANOUTCODE: <stage> done`.
pub const SENTINEL_PREFIX: &str = "FANOUTCODE: ";

/// The hand-back fixture, relative to the repo root (A2, A20).
pub const HANDBACK_FIXTURE: &str = "src/test/fixtures/handback.json";

/// The longest first line kept for a `Replied` chip, in characters.
const FIRST_LINE_CHARS: usize = 120;

/// What a `Stop` means (§6). Running background tasks: still working (`None`). The stage's sentinel as the last line
/// plus its artifact (when the stage has one): `StageDone`. Anything else, including a sentinel without the artifact:
/// `Replied` with the message's first line.
pub fn detect_handback(
    stage: Stage,
    last_message: &str,
    artifact_changed: bool,
    background_tasks: usize,
) -> Option<HandbackReason> {
    if background_tasks > 0 {
        return None;
    }
    if ends_with_sentinel(stage, last_message) && (stage.artifact().is_none() || artifact_changed) {
        return Some(HandbackReason::StageDone);
    }
    Some(HandbackReason::Replied {
        first_line: first_line(last_message),
    })
}

/// The last non-blank line, trimmed and unwrapped from `` ` ``, `*` and `_`, is exactly the stage's sentinel.
pub fn ends_with_sentinel(stage: Stage, message: &str) -> bool {
    message
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.trim_matches(|c| c == '`' || c == '*' || c == '_').trim())
        .is_some_and(|l| l == stage.sentinel())
}

/// The first non-blank line with markdown markers stripped, clipped to 120 characters; empty for an empty message.
pub fn first_line(message: &str) -> String {
    let line = message
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(|c: char| matches!(c, '#' | '>' | '-' | '*' | '_' | '`') || c.is_whitespace())
                .trim_end_matches(|c: char| matches!(c, '*' | '_' | '`') || c.is_whitespace())
        })
        .find(|l| !l.is_empty())
        .unwrap_or("");
    line.chars().take(FIRST_LINE_CHARS).collect()
}

/// The chip text for a hand-back, exactly as amendment A2 fixes it.
pub fn chip_text(reason: &HandbackReason, stage: Stage) -> String {
    match reason {
        HandbackReason::StageDone => match stage {
            Stage::Spec => "Spec ready",
            Stage::Plan => "Plan written, go?",
            Stage::Build => "Build done",
            Stage::Review => "Review written",
            Stage::Test => "Tests passed",
            Stage::Commit => "Message drafted",
            Stage::Done => "Done",
        }
        .to_owned(),
        HandbackReason::Replied { first_line } => first_line.clone(),
        HandbackReason::PermissionNeeded => "Permission needed".to_owned(),
        HandbackReason::TestFailed { .. } => "Tests failed".to_owned(),
        HandbackReason::Error { .. } => "Error".to_owned(),
    }
}

/// Writes `HANDBACK_FIXTURE`: every reason × stage, `{reason, stage, text}`, pretty-printed (A20: 35 rows).
#[cfg(test)]
pub(crate) fn write_handback_fixture(path: &Path) -> crate::Result<()> {
    let reasons = [
        HandbackReason::StageDone,
        HandbackReason::Replied {
            first_line: "Which DB?".into(),
        },
        HandbackReason::PermissionNeeded,
        HandbackReason::TestFailed { exit_code: 1 },
        HandbackReason::Error { message: "boom".into() },
    ];
    let mut rows = Vec::new();
    for reason in &reasons {
        for stage in Stage::ALL {
            rows.push(serde_json::json!({
                "reason": serde_json::to_value(reason)?,
                "stage": serde_json::to_value(stage)?,
                "text": chip_text(reason, stage),
            }));
        }
    }
    let text = serde_json::to_string_pretty(&rows)? + "\n";
    crate::memory::store::write_atomic(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_plus_artifact_is_stage_done() {
        let msg = "I wrote the plan.\n\nFANOUTCODE: plan done\n";
        assert_eq!(
            detect_handback(Stage::Plan, msg, true, 0),
            Some(HandbackReason::StageDone)
        );
    }

    #[test]
    fn sentinel_survives_markdown_wrapping_and_trailing_blank_lines() {
        for msg in [
            "Done.\n**FANOUTCODE: build done**  \n\n",
            "Done.\n`FANOUTCODE: build done`",
            "Done.\n_FANOUTCODE: build done_\n",
        ] {
            assert_eq!(
                detect_handback(Stage::Build, msg, true, 0),
                Some(HandbackReason::StageDone),
                "{msg:?}"
            );
        }
    }

    #[test]
    fn sentinel_quoted_mid_message_is_a_reply() {
        let msg = "I will end with FANOUTCODE: plan done once you answer:\nShould I use SQLite?";
        assert_eq!(
            detect_handback(Stage::Plan, msg, true, 0),
            Some(HandbackReason::Replied {
                first_line: "I will end with FANOUTCODE: plan done once you answer:".into()
            })
        );
    }

    #[test]
    fn wrong_stage_sentinel_is_a_reply() {
        assert!(matches!(
            detect_handback(Stage::Build, "FANOUTCODE: plan done", true, 0),
            Some(HandbackReason::Replied { .. })
        ));
    }

    #[test]
    fn sentinel_without_artifact_is_a_reply_where_an_artifact_is_required() {
        assert!(matches!(
            detect_handback(Stage::Review, "FANOUTCODE: review done", false, 0),
            Some(HandbackReason::Replied { .. })
        ));
        assert_eq!(
            detect_handback(Stage::Spec, "FANOUTCODE: spec done", false, 0),
            Some(HandbackReason::StageDone)
        );
    }

    #[test]
    fn running_background_tasks_mean_still_working() {
        assert_eq!(detect_handback(Stage::Build, "FANOUTCODE: build done", true, 2), None);
    }

    #[test]
    fn no_sentinel_is_a_reply_with_the_first_line() {
        assert_eq!(
            detect_handback(Stage::Plan, "\n## Question\nWhich DB?", true, 0),
            Some(HandbackReason::Replied {
                first_line: "Question".into()
            })
        );
        assert_eq!(
            detect_handback(Stage::Plan, "", true, 0),
            Some(HandbackReason::Replied {
                first_line: String::new()
            })
        );
        assert_eq!(first_line(&"x".repeat(200)).len(), 120);
    }

    #[test]
    fn chip_texts_match_amendment_a2() {
        let done: Vec<String> = Stage::ALL
            .iter()
            .map(|s| chip_text(&HandbackReason::StageDone, *s))
            .collect();
        assert_eq!(
            done,
            [
                "Spec ready",
                "Plan written, go?",
                "Build done",
                "Review written",
                "Tests passed",
                "Message drafted",
                "Done"
            ]
        );
        assert_eq!(
            chip_text(
                &HandbackReason::Replied {
                    first_line: "Which DB?".into()
                },
                Stage::Plan
            ),
            "Which DB?"
        );
        assert_eq!(
            chip_text(&HandbackReason::PermissionNeeded, Stage::Build),
            "Permission needed"
        );
        assert_eq!(
            chip_text(&HandbackReason::TestFailed { exit_code: 1 }, Stage::Test),
            "Tests failed"
        );
        assert_eq!(
            chip_text(&HandbackReason::Error { message: "boom".into() }, Stage::Build),
            "Error"
        );
    }

    #[test]
    fn handback_fixture_is_written_for_the_typescript_mirror() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(HANDBACK_FIXTURE);
        write_handback_fixture(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let rows = v.as_array().unwrap();
        assert_eq!(rows.len(), 35);
        assert_eq!(
            rows[1],
            serde_json::json!({ "reason": { "type": "StageDone" }, "stage": "Plan", "text": "Plan written, go?" })
        );
        assert_eq!(
            rows[7],
            serde_json::json!({ "reason": { "type": "Replied", "first_line": "Which DB?" }, "stage": "Spec", "text": "Which DB?" })
        );
        assert_eq!(
            rows[25]["reason"],
            serde_json::json!({ "type": "TestFailed", "exit_code": 1 })
        );
        assert_eq!(rows[25]["text"], "Tests failed");
    }
}
