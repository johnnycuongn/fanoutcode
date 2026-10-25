//! The spec watcher (§6.4, §7.4, A15): during Review, debounced on git changes, it reads the diff and `spec.md` and
//! returns a rewritten `spec.md` that describes what was built. The app (never the model) writes the file:
//! livespec-core's CLI provider allows only read-only tools, which is narrower than the spec's `Edit` on `spec.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use livespec_core::llm::{ChatRequest, Provider, Usage};
use livespec_core::usage::UsageLedger;
use serde::{Deserialize, Serialize};

use super::{call, tag_for};
use crate::changes::ChangeSet;
use crate::domain::{Stage, Task, TaskId};
use crate::git::{self, FileStatus, WatchHit};
use crate::Result;

/// Quiet time after the last tree change in Review before the watcher runs.
pub const DEBOUNCE_MS: u64 = 2_000;

pub const SPEC_WATCHER_SYSTEM: &str = "You keep a task's spec in step with what was actually built. You get the \
current spec.md and the diff of the work. Rewrite only the spec, so it describes what was built: keep the author's \
voice, structure and wording wherever they still hold, and change only what the code now contradicts or adds. You \
may read the repository to check details. Reply with exactly UNCHANGED when the spec still describes the work. \
Otherwise reply with one line `REASON: <what changed, in a few words>`, then a line `---`, then the full new spec.md \
and nothing after it.";

/// Run only in Review, and only when something outside the memory folder changed (Review Focus 4).
pub fn should_trigger(stage: Stage, hits: &[WatchHit]) -> bool {
    stage == Stage::Review && hits.contains(&WatchHit::Tree)
}

/// At most `max` characters, cut at a character boundary, marked when cut.
fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push_str("\n… (diff clipped)");
    out
}

/// The request: spec and per-repo diffs inlined (diffs clipped to half the provider's budget), read-only repo tools
/// from the home root, at most 12 turns.
pub fn request(spec_md: &str, diffs: &[(String, String)], home_root: &Path, provider: &Provider) -> ChatRequest {
    let budget = provider.context_budget_chars() / 2;
    let mut diff_text = String::new();
    for (repo, diff) in diffs {
        diff_text.push_str(&format!("<diff repo=\"{repo}\">\n{diff}\n</diff>\n"));
    }
    ChatRequest {
        system: SPEC_WATCHER_SYSTEM.to_owned(),
        user: format!("<spec>\n{spec_md}\n</spec>\n\n{}", clip(&diff_text, budget)),
        max_tokens: 8_000,
        cwd: Some(home_root.to_owned()),
        allow_repo_tools: true,
        max_turns: Some(12),
    }
}

/// What the watcher decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecWatchOutcome {
    Unchanged,
    Rewrite { spec_md: String, reason: String },
}

/// `UNCHANGED`, or `REASON: …` / `---` / the spec. Anything else, or a spec equal to the current one, is
/// `Unchanged`, so a malformed answer never overwrites the developer's spec.
pub fn parse(answer: &str, current_spec: &str) -> SpecWatchOutcome {
    let text = answer.trim_start();
    if text.lines().next().is_some_and(|l| l.trim() == "UNCHANGED") {
        return SpecWatchOutcome::Unchanged;
    }
    let Some((head, spec)) = text.split_once("\n---\n") else {
        return SpecWatchOutcome::Unchanged;
    };
    let Some(reason) = head.lines().find_map(|l| l.trim().strip_prefix("REASON:")) else {
        return SpecWatchOutcome::Unchanged;
    };
    if spec.trim().is_empty() || spec.trim_end() == current_spec.trim_end() {
        return SpecWatchOutcome::Unchanged;
    }
    SpecWatchOutcome::Rewrite {
        spec_md: spec.to_owned(),
        reason: reason.trim().to_owned(),
    }
}

/// The diff of each repo's task files against its base, untracked files shown whole.
pub async fn diffs(set: &ChangeSet) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for r in set.repos.iter().filter(|r| !r.files.is_empty()) {
        let paths: Vec<&str> = r.files.iter().map(|f| f.path.as_str()).collect();
        let mut args = vec!["diff", r.base.as_deref().unwrap_or("HEAD"), "--"];
        args.extend(&paths);
        let mut diff = git::run(&r.repo_root, &args).await.unwrap_or_default();
        for f in r
            .files
            .iter()
            .filter(|f| f.status == FileStatus::Untracked && !f.binary)
        {
            if let Ok(text) = tokio::fs::read_to_string(r.repo_root.join(&f.path)).await {
                diff.push_str(&format!("\n--- /dev/null\n+++ b/{}\n{}", f.path, text));
            }
        }
        out.push((r.repo_root.display().to_string(), diff));
    }
    out
}

/// One run: the diff, the request, the answer parsed. Returns the call's usage too.
pub async fn run_with_usage(
    provider: &Provider,
    ledger: Arc<UsageLedger>,
    task: &Task,
    set: &ChangeSet,
    spec_md: &str,
) -> Result<(SpecWatchOutcome, Vec<Usage>)> {
    let home: PathBuf = set
        .repos
        .first()
        .map(|r| r.repo_root.clone())
        .or_else(|| task.worktree_path.clone())
        .unwrap_or_else(|| task.memory_dir.clone());
    let diffs = diffs(set).await;
    if diffs.iter().all(|(_, d)| d.trim().is_empty()) {
        return Ok((SpecWatchOutcome::Unchanged, Vec::new()));
    }
    let out = call(
        provider,
        &request(spec_md, &diffs, &home, provider),
        ledger,
        tag_for(&home),
    )
    .await?;
    Ok((parse(&out.text, spec_md), out.usage))
}

pub async fn run(
    provider: &Provider,
    ledger: Arc<UsageLedger>,
    task: &Task,
    set: &ChangeSet,
    spec_md: &str,
) -> Result<SpecWatchOutcome> {
    Ok(run_with_usage(provider, ledger, task, set, spec_md).await?.0)
}

/// The payload of event `spec-watcher` (A15), once per applied rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct SpecWatcherEvent {
    pub task_id: TaskId,
    pub at: DateTime<Utc>,
    pub reason: String,
}

/// `Some` only for a rewrite.
pub fn event_for(task_id: &TaskId, at: DateTime<Utc>, outcome: &SpecWatchOutcome) -> Option<SpecWatcherEvent> {
    match outcome {
        SpecWatchOutcome::Rewrite { reason, .. } => Some(SpecWatcherEvent {
            task_id: task_id.clone(),
            at,
            reason: reason.clone(),
        }),
        SpecWatchOutcome::Unchanged => None,
    }
}

#[cfg(test)]
mod tests {
    use livespec_core::llm::OpenAiCompat;

    use super::*;

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    fn local_provider() -> Provider {
        Provider::OpenAiCompat(OpenAiCompat {
            base_url: "http://127.0.0.1:9".into(),
            model: "qwen3-4b-instruct".into(),
            context_tokens: 16_384,
            http: reqwest::Client::new(),
        })
    }

    #[test]
    fn request_inlines_spec_and_diff_and_allows_read_only_repo_tools() {
        let r = request(
            "# Spec\nRows per project.",
            &[("/ws/app".into(), "diff --git a/x b/x\n+row".into())],
            Path::new("/ws/app"),
            &local_provider(),
        );
        assert!(r.user.contains("Rows per project.") && r.user.contains("+row"));
        assert!(r.allow_repo_tools && r.cwd.as_deref() == Some(Path::new("/ws/app")) && r.max_turns == Some(12));
    }

    #[test]
    fn unchanged_answer_is_unchanged() {
        assert_eq!(parse("UNCHANGED\n", "# Spec"), SpecWatchOutcome::Unchanged);
    }

    #[test]
    fn a_rewrite_carries_reason_and_new_spec() {
        let out = parse(
            "REASON: rows are now collapsible\n---\n# Spec\nRows collapse.\n",
            "# Spec\nRows per project.",
        );
        assert_eq!(
            out,
            SpecWatchOutcome::Rewrite {
                spec_md: "# Spec\nRows collapse.\n".into(),
                reason: "rows are now collapsible".into()
            }
        );
    }

    #[test]
    fn an_identical_rewrite_is_dropped() {
        assert_eq!(
            parse("REASON: nothing\n---\n# Spec\n", "# Spec\n"),
            SpecWatchOutcome::Unchanged
        );
    }

    #[test]
    fn changes_only_under_the_memory_dir_do_not_trigger() {
        assert!(!should_trigger(Stage::Review, &[WatchHit::MemoryOnly]));
        assert!(should_trigger(Stage::Review, &[WatchHit::MemoryOnly, WatchHit::Tree]));
        assert!(!should_trigger(Stage::Build, &[WatchHit::Tree]));
    }

    #[test]
    fn only_an_applied_rewrite_yields_a_spec_watcher_event() {
        let at = Utc::now();
        let rewrite = SpecWatchOutcome::Rewrite {
            spec_md: "# Spec\n".into(),
            reason: "rows collapse".into(),
        };
        assert_eq!(
            event_for(&tid(), at, &rewrite),
            Some(SpecWatcherEvent {
                task_id: tid(),
                at,
                reason: "rows collapse".into()
            })
        );
        assert_eq!(event_for(&tid(), at, &SpecWatchOutcome::Unchanged), None);
    }
}
