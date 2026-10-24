//! Headless helpers (§7.4): short single-turn calls through livespec-core's providers, metered by its usage
//! ledger. `HelperKind` is the payload of `Effect::RunHelper` (Plan 03); the shell's `helpers::run` dispatches it.

#[cfg(test)]
pub(crate) mod test_server;

use std::path::Path;
use std::sync::Arc;

use livespec_core::llm::{ChatRequest, Chunk, Provider, Usage};
use livespec_core::usage::{sent_chars, Meter, UsageKind, UsageLedger, UsageTag};
use serde::{Deserialize, Serialize};

use crate::Result;

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

/// A helper's answer and the usage the provider reported (for the task's cost ledger).
#[derive(Debug, Clone, PartialEq)]
pub struct HelperOutput {
    pub text: String,
    pub usage: Vec<Usage>,
}

/// One single-turn call. One `Meter` observes every chunk, so LiveSpec's ledger stays authoritative (an aborted or
/// unreported call is estimated there); `Chunk::Usage` is also returned for Team Code's per-task cost.
pub async fn call(
    provider: &Provider,
    req: &ChatRequest,
    ledger: Arc<UsageLedger>,
    tag: UsageTag,
) -> Result<HelperOutput> {
    let mut meter = Meter::new(ledger, tag, provider.model().to_owned(), sent_chars(req));
    let (mut text, mut usage) = (String::new(), Vec::new());
    let result = provider
        .stream(req, &mut |c: Chunk| {
            meter.observe(&c);
            match c {
                Chunk::Text(t) => text.push_str(&t),
                Chunk::Usage(u) => usage.push(u),
                Chunk::Activity(_) => {}
            }
        })
        .await;
    meter.finish(result.is_ok());
    result?;
    Ok(HelperOutput { text, usage })
}

/// Helper calls are filed under the project root as notes in LiveSpec's ledger.
pub fn tag_for(project_root: &Path) -> UsageTag {
    UsageTag {
        root: Some(project_root.display().to_string()),
        kind: UsageKind::Notes,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use livespec_core::llm::{ChatRequest, OpenAiCompat, Provider};
    use livespec_core::usage::{UsageKind, UsageLedger, UsageTag};

    use super::test_server::{sse_body, sse_server};
    use super::*;

    fn req(system: &str, user: &str) -> ChatRequest {
        ChatRequest {
            system: system.into(),
            user: user.into(),
            max_tokens: 50,
            cwd: None,
            allow_repo_tools: false,
            max_turns: None,
        }
    }

    #[tokio::test]
    async fn call_returns_the_text_and_captures_usage() {
        let with_usage = sse_body("Hi", 7, 2);
        let (url, _server) = sse_server(&with_usage).await; // replies "Hi" with usage {prompt_tokens: 7, completion_tokens: 2}
        let provider = Provider::OpenAiCompat(OpenAiCompat {
            base_url: url,
            model: "qwen3-4b-instruct".into(),
            context_tokens: 4096,
            http: reqwest::Client::new(),
        });
        let dir = tempfile::tempdir().unwrap();
        let ledger = Arc::new(UsageLedger::new(dir.path().join("usage.json")));
        let out = call(
            &provider,
            &req("sys", "user"),
            ledger.clone(),
            tag_for(Path::new("/repo")),
        )
        .await
        .unwrap();
        assert_eq!(out.text, "Hi");
        assert_eq!((out.usage.len(), out.usage[0].input, out.usage[0].output), (1, 7, 2));
        assert_eq!(ledger.report(0).iter().map(|b| b.runs).sum::<u64>(), 1);
    }

    #[test]
    fn tag_for_files_under_the_project_root() {
        assert_eq!(
            tag_for(Path::new("/repo")),
            UsageTag {
                root: Some("/repo".into()),
                kind: UsageKind::Notes
            }
        );
    }
}
