//! Project briefs (§7.4): LiveSpec's one-time brief per project, built lazily and reused for the Linked projects
//! section of every prompt.

use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use livespec_core::llm::Provider;
use livespec_core::project::explore::brief_source;
use livespec_core::project::{scan, BriefStore, StoredBrief};
use livespec_core::prompt::brief_request;
use livespec_core::usage::{UsageKind, UsageLedger, UsageTag};

use super::call;
use crate::Result;

/// The stored brief's text (fresh or stale), with no model call.
pub fn cached(store: &BriefStore, root: &Path) -> Option<String> {
    store.load(root).map(|b| b.text)
}

/// The stored brief, fresh or stale; else scans the project, asks for a brief and saves it.
pub async fn ensure(
    store: &BriefStore,
    root: &Path,
    provider: &Provider,
    ledger: Arc<UsageLedger>,
) -> Result<StoredBrief> {
    if let Some(b) = store.load(root) {
        return Ok(b);
    }
    let index = scan(root)?;
    let req = brief_request(&index, provider);
    let tag = UsageTag {
        root: Some(root.display().to_string()),
        kind: UsageKind::Brief,
    };
    let out = call(provider, &req, ledger, tag).await?;
    let brief = StoredBrief {
        root: root.display().to_string(),
        fingerprint: index.fingerprint,
        text: out.text.trim().to_owned(),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()),
        source: brief_source(provider.model(), 1),
    };
    store.save(&brief)?;
    Ok(brief)
}

#[cfg(test)]
mod tests {
    use livespec_core::llm::ClaudeCli;

    use super::*;

    #[test]
    fn cached_brief_is_returned_without_a_model_call() {
        let dir = tempfile::tempdir().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));
        assert_eq!(cached(&store, Path::new("/repo")), None);
        store
            .save(&StoredBrief {
                root: "/repo".into(),
                fingerprint: "f".into(),
                text: "A Tauri app.".into(),
                created_at: 1,
                source: "test".into(),
            })
            .unwrap();
        assert_eq!(cached(&store, Path::new("/repo")).as_deref(), Some("A Tauri app."));
    }

    #[tokio::test]
    async fn ensure_builds_only_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let store = BriefStore::new(dir.path().join("briefs"));
        let root = dir.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        store
            .save(&StoredBrief {
                root: root.display().to_string(),
                fingerprint: "stale".into(),
                text: "old".into(),
                created_at: 1,
                source: "t".into(),
            })
            .unwrap();
        let broken = Provider::ClaudeCli(ClaudeCli::new("/nonexistent/claude".into(), "haiku", None)); // would fail if called
        let b = ensure(
            &store,
            &root,
            &broken,
            Arc::new(UsageLedger::new(dir.path().join("u.json"))),
        )
        .await
        .unwrap();
        assert_eq!(b.text, "old");
    }
}
