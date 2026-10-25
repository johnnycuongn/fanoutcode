//! A cached one-paragraph brief per project, `<app_data>/projects/<project-id>/brief.md`, for the Linked projects
//! block.

use std::path::{Path, PathBuf};

use crate::domain::ProjectId;
use crate::memory::store::write_atomic;
use crate::Result;

/// `<app_data>/projects/<project-id>/brief.md`, the id re-validated.
pub fn brief_path(app_data: &Path, project_id: &ProjectId) -> Result<PathBuf> {
    let id = ProjectId::parse(project_id.as_str())?;
    Ok(app_data.join("projects").join(id.as_str()).join("brief.md"))
}

pub fn cached_brief(app_data: &Path, project_id: &ProjectId) -> Option<String> {
    let text = std::fs::read_to_string(brief_path(app_data, project_id).ok()?).ok()?;
    Some(text.trim().to_owned()).filter(|t| !t.is_empty())
}

pub fn store_brief(app_data: &Path, project_id: &ProjectId, text: &str) -> Result<()> {
    write_atomic(&brief_path(app_data, project_id)?, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brief_cache_round_trips_and_validates_id() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = ProjectId::parse("p-1").unwrap();
        assert!(cached_brief(dir.path(), &p1).is_none());
        store_brief(dir.path(), &p1, "A brief.").unwrap();
        assert_eq!(cached_brief(dir.path(), &p1).as_deref(), Some("A brief."));
        let bad: ProjectId = serde_json::from_value(serde_json::json!("../x")).unwrap();
        assert!(brief_path(dir.path(), &bad).is_err());
    }
}
