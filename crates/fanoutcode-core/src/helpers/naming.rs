//! The naming helper (§7.4): a 3 to 6 word title from the intent, on the fast tier.

use std::sync::Arc;

use livespec_core::llm::{ChatRequest, Provider, Usage};
use livespec_core::usage::{UsageKind, UsageLedger, UsageTag};

use super::call;
use crate::Result;

pub const NAMING_SYSTEM: &str =
    "You name engineering tasks. Reply with a title of three to six words, Title Case, no quotes, no trailing period, nothing else.";

/// The single-turn request: the intent alone, no tools, a short answer.
pub fn request(intent: &str) -> ChatRequest {
    ChatRequest {
        system: NAMING_SYSTEM.to_owned(),
        user: intent.trim().to_owned(),
        max_tokens: 32,
        cwd: None,
        allow_repo_tools: false,
        max_turns: None,
    }
}

/// The answer's first non-empty line with quotes, backticks and a trailing period stripped, when it has 3 to 6 words.
pub fn accept(answer: &str) -> Option<String> {
    let quotes: &[char] = &['"', '\'', '`', '“', '”'];
    let line = answer.lines().map(str::trim).find(|l| !l.is_empty())?;
    let title = line
        .trim_matches(quotes)
        .trim()
        .trim_end_matches('.')
        .trim()
        .trim_matches(quotes)
        .trim();
    let words = title.split_whitespace().count();
    (3..=6)
        .contains(&words)
        .then(|| title.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// The first six words of the intent's first non-empty line.
pub fn fallback(intent: &str) -> String {
    let line = intent.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    line.split_whitespace().take(6).collect::<Vec<_>>().join(" ")
}

/// The model's title, retried once; `None` when neither answer fits. Returns every call's usage.
pub async fn run_with_usage(
    provider: &Provider,
    ledger: Arc<UsageLedger>,
    tag: UsageTag,
    intent: &str,
) -> (Option<String>, Vec<Usage>) {
    let req = request(intent);
    let mut usage = Vec::new();
    for _ in 0..2 {
        if let Ok(out) = call(provider, &req, ledger.clone(), tag.clone()).await {
            usage.extend(out.usage);
            if let Some(title) = accept(&out.text) {
                return (Some(title), usage);
            }
        }
    }
    (None, usage)
}

/// A title for the intent: the model's (one retry), else `fallback`.
pub async fn run(provider: &Provider, ledger: Arc<UsageLedger>, intent: &str) -> Result<String> {
    let tag = UsageTag {
        root: None,
        kind: UsageKind::Notes,
    };
    Ok(run_with_usage(provider, ledger, tag, intent)
        .await
        .0
        .unwrap_or_else(|| fallback(intent)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naming_request_asks_for_three_to_six_words() {
        let r = request("Add swimlanes to the workspace board");
        assert!(r.system.contains("three to six words"));
        assert!(r.user.contains("swimlanes"));
        assert!(!r.allow_repo_tools);
        assert_eq!(r.max_tokens, 32);
    }

    #[test]
    fn accept_trims_quotes_and_rejects_wrong_lengths() {
        assert_eq!(
            accept("\"Workspace Board Swimlanes.\"\n"),
            Some("Workspace Board Swimlanes".into())
        );
        assert_eq!(accept("Swimlanes"), None);
        assert_eq!(accept("one two three four five six seven"), None);
    }

    #[test]
    fn fallback_is_the_first_six_words_of_the_intent() {
        assert_eq!(
            fallback("\n\nAdd swimlanes to the workspace board so that rows group by project"),
            "Add swimlanes to the workspace board"
        );
    }
}
