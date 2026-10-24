//! The key registry: one zero-sized marker per row, `KEYS` in A12 order, and `ResolvedSettings`, all generated from
//! the one table below. Plans 04 and 10 insert their rows (`terminal_theme` after `cost_overview`; `jira` and
//! `jira_transitions` after `scrollback_lines`) as one line each.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{resolve, values, Scope, ScopeKind, ScopeValues, SettingKey};
use crate::domain;

/// One registry row: where the key may be set, how to check a value, and the type named in errors.
pub struct KeyInfo {
    pub key: &'static str,
    pub levels: &'static [ScopeKind],
    pub check: fn(&Value) -> bool,
    /// The value type as written in the table, e.g. `Option<domain::WorktreeConvention>`.
    pub expected: &'static str,
}

macro_rules! settings_table {
    // the value type is captured as raw tokens in parentheses: specta's derive cannot read an opaque `ty` fragment
    ($( $marker:ident, $key:ident, [$($level:ident),+], ($($ty:tt)+), $default:expr; )+) => {
        $(
            #[doc = concat!("The `", stringify!($key), "` setting.")]
            pub struct $marker;

            impl SettingKey for $marker {
                const KEY: &'static str = stringify!($key);
                const LEVELS: &'static [ScopeKind] = &[$(ScopeKind::$level),+];
                type Value = $($ty)+;
                fn default_value() -> $($ty)+ {
                    $default
                }
            }
        )+

        /// Every registered key, in A12 order.
        pub const KEYS: &[KeyInfo] = &[$(
            KeyInfo {
                key: stringify!($key),
                levels: &[$(ScopeKind::$level),+],
                check: |v| serde_json::from_value::<$($ty)+>(v.clone()).is_ok(),
                expected: stringify!($($ty)+),
            },
        )+];

        /// Every key resolved for one scope; fields are the key names as written (A16).
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
        pub struct ResolvedSettings {
            $( pub $key: $($ty)+, )+
        }

        /// Resolves every key at `scope`.
        pub fn resolve_all(values: &ScopeValues, scope: &Scope) -> ResolvedSettings {
            ResolvedSettings { $( $key: resolve::<$marker>(values, scope), )+ }
        }
    };
}

settings_table! {
    Theme, theme, [App], (domain::ThemePreference), domain::ThemePreference::System;
    Accent, accent, [App], (values::Accent), values::Accent::Blue;
    DetailLevel, detail_level, [App, Project], (domain::DetailLevel), domain::DetailLevel::Simple;
    DockPosition, dock_position, [App], (values::DockPosition), values::DockPosition::Bottom;
    TeamViewArrangement, team_view_arrangement, [App], (values::TeamArrangement), values::TeamArrangement::HubGraph;
    TeamViewPlacement, team_view_placement, [App], (values::TeamPlacement), values::TeamPlacement::DockGrows;
    TeamCanvasLayout, team_canvas_layout, [App], (values::CanvasLayout), values::CanvasLayout::AutoReflow;
    TeamDrillInLens, team_drill_in_lens, [App], (values::DrillInLens), values::DrillInLens::ByFile;
    WorkspaceProjectLayer, workspace_project_layer, [App, Workspace], (values::ProjectLayer), values::ProjectLayer::MultiRoot;
    WorkspaceBoard, workspace_board, [App, Workspace], (values::WorkspaceBoard), values::WorkspaceBoard::Swimlanes;
    Worktree, worktree, [Project, Task], (domain::WorktreeChoice), domain::WorktreeChoice::Off;
    WorktreeConvention, worktree_convention, [App, Project], (Option<domain::WorktreeConvention>), None;
    WorktreesDir, worktrees_dir, [Project], (Option<String>), None;
    MayCommit, may_commit, [Project], (bool), false;
    TestCommand, test_command, [Project], (Option<String>), None;
    TestEnabled, test_enabled, [Project, Task], (bool), true;
    AdvanceOnGreen, advance_on_green, [Project], (bool), false;
    LivespecLevel, livespec_level, [Project], (domain::LiveSpecLevel), domain::LiveSpecLevel::Off;
    AgentKind, agent_kind, [Project], (domain::AgentKind), domain::AgentKind::ClaudeCode;
    ModelProvider, model_provider, [App], (values::ModelProvider), values::ModelProvider::ClaudeCli;
    CostOverview, cost_overview, [App], (bool), false;
    TerminalFontFamily, terminal_font_family, [App], (String), "JetBrains Mono".to_owned();
    TerminalFontSize, terminal_font_size, [App], (u32), 13;
    TerminalLigatures, terminal_ligatures, [App], (bool), false;
    ScrollbackLines, scrollback_lines, [App], (u32), 10_000;
    FirstRunComplete, first_run_complete, [App], (bool), false;
}

/// The registry row for `key`.
pub fn info(key: &str) -> Option<&'static KeyInfo> {
    KEYS.iter().find(|k| k.key == key)
}
