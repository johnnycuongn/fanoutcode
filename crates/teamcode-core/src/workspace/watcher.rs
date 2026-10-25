//! Watches workspace files saved outside app data (explicit-save mode) for edits made elsewhere.

use crate::domain::WorkspaceId;

/// Built out in Task 4; the store already calls `note_own_write` before each write.
pub struct WorkspaceFileWatcher {}

impl WorkspaceFileWatcher {
    pub fn note_own_write(&self, _id: &WorkspaceId, _bytes: &[u8]) {}
}
