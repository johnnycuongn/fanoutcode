//! Fixtures shared by the crate's tests.

use std::path::PathBuf;

use crate::domain::{MemoryLocation, Project, ProjectColor, ProjectId};

/// A project rooted at `/repos/<id>`, named `id`, coloured `indigo`.
pub fn project_fixture(id: &str, loc: MemoryLocation) -> Project {
    Project {
        id: ProjectId::parse(id).unwrap(),
        name: id.to_owned(),
        root: PathBuf::from(format!("/repos/{id}")),
        color: ProjectColor("indigo".into()),
        memory_location: loc,
        created_at: chrono::Utc::now(),
    }
}
