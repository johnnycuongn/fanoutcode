//! Context docs: files, folders, URLs and Confluence pages listed per project or workspace (§12).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// What a context doc points at. Serialises as `"File"`, `"Folder"`, `"Url"`, `"Confluence"` (A16), the strings
/// stored in `context_docs.kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ContextDocKind {
    File,
    Folder,
    Url,
    Confluence,
}

/// One reference listed in prompts under "Project references" or "Workspace references".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct ContextDoc {
    pub id: String,
    pub kind: ContextDocKind,
    pub target: String,
    pub label: String,
    pub page_id: Option<String>,
    pub fetched_at: Option<DateTime<Utc>>,
    pub order: u32,
}
