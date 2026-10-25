//! Setting value enums that are not domain types. Unit-only, so they serialise as their Rust variant names (A16).
//! `theme` uses `domain::ThemePreference` (Plan 01). `TerminalTheme` (Plan 04) and `JiraSite`, `JiraTransitions`
//! (Plan 10) are owned by those plans (A19), never defined here.

use serde::{Deserialize, Serialize};

pub use super::keys::ResolvedSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum Accent {
    Blue,
    Violet,
    Teal,
    Orange,
}

/// How a workspace window shows its projects in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ProjectLayer {
    MultiRoot,
    Rail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum WorkspaceBoard {
    Swimlanes,
    OneProject,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum DockPosition {
    Bottom,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum TeamArrangement {
    HubGraph,
    Flow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum TeamPlacement {
    DockGrows,
    Swap,
    OwnPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum CanvasLayout {
    AutoReflow,
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum DrillInLens {
    ByFile,
    EventStream,
}

/// LiveSpec's providers (`claude -p`, API, local); model tiers stay in LiveSpec's own config (A19).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ModelProvider {
    ClaudeCli,
    Api,
    Local,
}
