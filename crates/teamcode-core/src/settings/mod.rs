//! Scoped settings (§6.8, §13): task → project → workspace → app → default, most specific wins, each key settable
//! only at the levels its row allows. Pure; the SQLite side is `db::settings`.

pub mod keys;
pub mod values;

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{ProjectId, TaskId, WorkspaceId};
use crate::{Error, Result};

pub use keys::{resolve_all, KeyInfo, ResolvedSettings, KEYS};
pub use values::{
    Accent, CanvasLayout, DockPosition, DrillInLens, ModelProvider, ProjectLayer, TeamArrangement, TeamPlacement,
    WorkspaceBoard,
};

/// The layers a setting is resolved through. The contract's `app` layer is implicit and always present.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Scope {
    pub workspace: Option<WorkspaceId>,
    pub project: Option<ProjectId>,
    pub task: Option<TaskId>,
}

impl Scope {
    pub fn app() -> Scope {
        Scope::default()
    }

    /// The layer a write at this scope goes to: `(App, "")` when every layer is `None`.
    pub fn most_specific(&self) -> (ScopeKind, String) {
        if let Some(t) = &self.task {
            (ScopeKind::Task, t.as_str().to_owned())
        } else if let Some(p) = &self.project {
            (ScopeKind::Project, p.as_str().to_owned())
        } else if let Some(w) = &self.workspace {
            (ScopeKind::Workspace, w.as_str().to_owned())
        } else {
            (ScopeKind::App, String::new())
        }
    }
}

/// A settings layer; `as_str` is the `settings.scope_kind` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum ScopeKind {
    App,
    Workspace,
    Project,
    Task,
}

impl ScopeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ScopeKind::App => "app",
            ScopeKind::Workspace => "workspace",
            ScopeKind::Project => "project",
            ScopeKind::Task => "task",
        }
    }
}

/// The stored rows of each layer present in a scope, by key (`db::settings::load`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScopeValues {
    pub app: HashMap<String, Value>,
    pub workspace: HashMap<String, Value>,
    pub project: HashMap<String, Value>,
    pub task: HashMap<String, Value>,
}

/// One setting: its key, the levels it may be set at, its value type and default.
pub trait SettingKey {
    const KEY: &'static str;
    const LEVELS: &'static [ScopeKind];
    type Value: Serialize + DeserializeOwned + Clone + PartialEq;
    fn default_value() -> Self::Value;
}

/// Task → project → workspace → app → default, visiting only the levels `T` allows and the layers present in
/// `scope`. A stored value that does not deserialise is skipped.
pub fn resolve<T: SettingKey>(values: &ScopeValues, scope: &Scope) -> T::Value {
    let layers = [
        (ScopeKind::Task, scope.task.is_some(), &values.task),
        (ScopeKind::Project, scope.project.is_some(), &values.project),
        (ScopeKind::Workspace, scope.workspace.is_some(), &values.workspace),
        (ScopeKind::App, true, &values.app),
    ];
    layers
        .into_iter()
        .filter(|(kind, present, _)| *present && T::LEVELS.contains(kind))
        .find_map(|(_, _, map)| map.get(T::KEY).and_then(|v| serde_json::from_value(v.clone()).ok()))
        .unwrap_or_else(T::default_value)
}

/// Checks a write before it is stored. `null` is always accepted: it removes the override at that level.
pub fn validate(key: &str, level: ScopeKind, value: &Value) -> Result<()> {
    let info = keys::info(key).ok_or_else(|| Error::Invalid(format!("Unknown setting {key}")))?;
    if !info.levels.contains(&level) {
        return Err(Error::Invalid(format!(
            "{key} cannot be set at {} level",
            level.as_str()
        )));
    }
    if !value.is_null() && !(info.check)(value) {
        let expected = info.expected.replace("domain::", "").replace("values::", "");
        return Err(Error::Invalid(format!("{key}: expected {expected}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DetailLevel, WorktreeChoice};
    use crate::ids::{ProjectId, TaskId, WorkspaceId};
    use crate::Error;

    fn scope_all() -> Scope {
        Scope {
            workspace: Some(WorkspaceId::parse("ws-1a-1").unwrap()),
            project: Some(ProjectId::parse("p-1a-1").unwrap()),
            task: Some(TaskId::parse("t-1a-1").unwrap()),
        }
    }

    #[test]
    fn defaults_apply_when_nothing_is_stored() {
        let v = ScopeValues::default();
        assert_eq!(resolve::<keys::DetailLevel>(&v, &Scope::app()), DetailLevel::Simple);
        assert_eq!(resolve::<keys::ScrollbackLines>(&v, &Scope::app()), 10_000);
        assert_eq!(resolve::<keys::WorktreeConvention>(&v, &Scope::app()), None);
        assert!(resolve::<keys::TestEnabled>(&v, &scope_all()));
    }

    #[test]
    fn most_specific_allowed_level_wins() {
        let mut v = ScopeValues::default();
        v.app.insert("detail_level".into(), serde_json::json!("Detailed"));
        v.project.insert("detail_level".into(), serde_json::json!("Medium"));
        v.task.insert("detail_level".into(), serde_json::json!("Simple")); // task level is not allowed for detail_level
        assert_eq!(resolve::<keys::DetailLevel>(&v, &scope_all()), DetailLevel::Medium);
        assert_eq!(resolve::<keys::DetailLevel>(&v, &Scope::app()), DetailLevel::Detailed);
        v.task.insert("worktree".into(), serde_json::json!("On"));
        v.project.insert("worktree".into(), serde_json::json!("Off"));
        assert_eq!(resolve::<keys::Worktree>(&v, &scope_all()), WorktreeChoice::On);
    }

    #[test]
    fn layers_outside_the_scope_are_ignored() {
        let mut v = ScopeValues::default();
        v.workspace.insert("workspace_board".into(), serde_json::json!("Mixed"));
        let project_only = Scope {
            workspace: None,
            project: Some(ProjectId::parse("p-1a-1").unwrap()),
            task: None,
        };
        assert_eq!(
            resolve::<keys::WorkspaceBoard>(&v, &project_only),
            WorkspaceBoard::Swimlanes
        );
        assert_eq!(resolve::<keys::WorkspaceBoard>(&v, &scope_all()), WorkspaceBoard::Mixed);
    }

    #[test]
    fn a_malformed_stored_value_falls_through() {
        let mut v = ScopeValues::default();
        v.project.insert("detail_level".into(), serde_json::json!("medium")); // the old camelCase spelling no longer parses (A16)
        v.app.insert("detail_level".into(), serde_json::json!("Medium"));
        assert_eq!(resolve::<keys::DetailLevel>(&v, &scope_all()), DetailLevel::Medium);
    }

    #[test]
    fn validate_rejects_unknown_keys_wrong_levels_and_wrong_types() {
        assert!(
            matches!(validate("nope", ScopeKind::App, &serde_json::json!(1)), Err(Error::Invalid(m)) if m.contains("Unknown setting"))
        );
        assert!(
            matches!(validate("may_commit", ScopeKind::App, &serde_json::json!(true)), Err(Error::Invalid(m)) if m.contains("cannot be set at app"))
        );
        assert!(
            matches!(validate("may_commit", ScopeKind::Project, &serde_json::json!("yes")), Err(Error::Invalid(m)) if m.contains("expected"))
        );
        assert!(validate("may_commit", ScopeKind::Project, &serde_json::json!(true)).is_ok());
        assert!(validate("test_command", ScopeKind::Project, &serde_json::Value::Null).is_ok());
        assert!(validate("team_view_arrangement", ScopeKind::App, &serde_json::json!("Flow")).is_ok());
        assert!(
            matches!(validate("team_view_arrangement", ScopeKind::App, &serde_json::json!("flow")), Err(Error::Invalid(m)) if m.contains("expected"))
        );
        assert!(
            matches!(validate("link_policy", ScopeKind::Workspace, &serde_json::json!("ReadOnly")), Err(Error::Invalid(m)) if m.contains("Unknown setting"))
        );
        assert!(matches!(
            validate("terminal_font_size", ScopeKind::App, &serde_json::json!("big")),
            Err(Error::Invalid(_))
        ));
        // terminal_theme, jira and jira_transitions are validated by their owning plans' tests (Plan 04 Task 13, Plan 10 Task 13), which add the rows
    }

    #[test]
    fn registry_is_the_a12_keys_this_plan_owns_and_resolve_all_uses_defaults() {
        let names: Vec<&str> = KEYS.iter().map(|k| k.key).collect();
        // A12 order without the two rows Plan 10 inserts (jira, jira_transitions after scrollback_lines): 27 keys here, A12's 29
        // once Plan 10 adds its keys to this list in its own failing-test step (Plan 04 added terminal_theme)
        assert_eq!(
            names,
            [
                "theme",
                "accent",
                "detail_level",
                "dock_position",
                "team_view_arrangement",
                "team_view_placement",
                "team_canvas_layout",
                "team_drill_in_lens",
                "workspace_project_layer",
                "workspace_board",
                "worktree",
                "worktree_convention",
                "worktrees_dir",
                "may_commit",
                "test_command",
                "test_enabled",
                "advance_on_green",
                "livespec_level",
                "agent_kind",
                "model_provider",
                "cost_overview",
                "terminal_theme",
                "terminal_font_family",
                "terminal_font_size",
                "terminal_ligatures",
                "scrollback_lines",
                "first_run_complete"
            ]
        );
        let r = resolve_all(&ScopeValues::default(), &Scope::app());
        assert_eq!(r.dock_position, DockPosition::Bottom);
        assert_eq!(
            (r.workspace_board, r.workspace_project_layer),
            (WorkspaceBoard::Swimlanes, ProjectLayer::MultiRoot)
        );
        assert_eq!(
            (
                r.terminal_font_family.as_str(),
                r.terminal_font_size,
                r.terminal_ligatures,
                r.scrollback_lines
            ),
            ("JetBrains Mono", 13, false, 10_000)
        );
        assert_eq!(r.model_provider, ModelProvider::ClaudeCli);
        assert!(!r.first_run_complete);
        assert!(!r.cost_overview && !r.may_commit && r.test_command.is_none());
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["team_view_arrangement"], "HubGraph"); // A16: field names as written, variants as Rust names
        assert_eq!(json["model_provider"], "ClaudeCli");
    }

    #[test]
    fn most_specific_names_the_layer_to_write() {
        assert_eq!(Scope::app().most_specific(), (ScopeKind::App, String::new()));
        assert_eq!(scope_all().most_specific(), (ScopeKind::Task, "t-1a-1".into()));
    }
}
