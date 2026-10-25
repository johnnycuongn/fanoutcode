//! The Jira settings keys: the linked site (workspace default, project override) and the optional stage-to-status
//! transitions (project only).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::Stage;

pub use crate::settings::keys::{Jira as JiraSiteKey, JiraTransitions as JiraTransitionsKey};

/// Moves the issue when the task enters a stage: stage word (`"review"`) → transition name (`"In Review"`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JiraTransitions {
    pub enabled: bool,
    pub on_stage: BTreeMap<String, String>,
}

/// The transition to apply on entering `stage`; `None` when disabled or unmapped.
pub fn transition_for(cfg: &JiraTransitions, stage: Stage) -> Option<&str> {
    if !cfg.enabled {
        return None;
    }
    cfg.on_stage.get(stage.as_str()).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ids::{ProjectId, WorkspaceId};
    use crate::settings::{keys::KEYS, resolve, resolve_all, validate, Scope, ScopeKind, ScopeValues};
    use crate::workspace::test_support::fixture;
    use serde_json::json;

    fn btree(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(a, b)| ((*a).into(), (*b).into())).collect()
    }

    #[test]
    fn project_without_site_inherits_workspace_default() {
        let fx = fixture();
        let ws = WorkspaceId::parse("ws-1").unwrap();
        let conn = fx.db.lock().unwrap();
        db::settings::set(&conn, ScopeKind::Workspace, ws.as_str(), "jira",
            &json!({"site_url": "https://acme.atlassian.net", "email": "me@acme.com", "account": "jira:workspace:ws-1"})).unwrap();
        let scope = Scope {
            workspace: Some(ws.clone()),
            project: Some(fx.api.id.clone()),
            task: None,
        };
        let values = db::settings::load(&conn, &scope).unwrap();
        assert_eq!(
            resolve::<JiraSiteKey>(&values, &scope).map(|s| s.account),
            Some("jira:workspace:ws-1".into())
        );
        let alone = Scope {
            project: Some(fx.web.id.clone()),
            ..Default::default()
        };
        assert!(resolve::<JiraSiteKey>(&db::settings::load(&conn, &alone).unwrap(), &alone).is_none());
    }

    #[test]
    fn jira_keys_are_registered_in_their_a12_slots_resolve_to_defaults_and_jira_inherits_from_the_workspace() {
        let names: Vec<&str> = KEYS.iter().map(|k| k.key).collect();
        let at = names.iter().position(|k| *k == "jira").expect("jira registered");
        assert_eq!(
            names[at - 1..=at + 2],
            ["scrollback_lines", "jira", "jira_transitions", "first_run_complete"]
        );
        let scope = Scope {
            workspace: Some(WorkspaceId::parse("ws-1a-1").unwrap()),
            project: Some(ProjectId::parse("p-1a-1").unwrap()),
            task: None,
        };
        let empty = ScopeValues::default();
        assert_eq!(resolve::<JiraSiteKey>(&empty, &scope), None);
        assert_eq!(
            resolve::<JiraTransitionsKey>(&empty, &scope),
            JiraTransitions {
                enabled: false,
                on_stage: BTreeMap::new()
            }
        );
        let r = resolve_all(&empty, &scope);
        assert_eq!(
            (
                r.jira.clone(),
                serde_json::to_value(&r).unwrap()["jira_transitions"].clone()
            ),
            (None, json!({"enabled": false, "on_stage": {}}))
        );
        let mut v = ScopeValues::default();
        v.workspace.insert("jira".into(), json!({"site_url": "https://acme.atlassian.net", "email": "me@acme.com", "account": "jira:workspace:ws-1a-1"}));
        assert_eq!(
            resolve::<JiraSiteKey>(&v, &scope).map(|s| s.account),
            Some("jira:workspace:ws-1a-1".into())
        );
        assert!(validate("jira", ScopeKind::Workspace, &v.workspace["jira"]).is_ok());
        assert!(validate("jira", ScopeKind::App, &json!(null)).is_err()); // workspace and project only
        assert!(validate(
            "jira_transitions",
            ScopeKind::Workspace,
            &json!({"enabled": true, "on_stage": {}})
        )
        .is_err()); // project only
    }

    #[test]
    fn transition_for_respects_enabled_and_stage() {
        let cfg = JiraTransitions {
            enabled: true,
            on_stage: btree(&[("review", "In Review"), ("done", "Done")]),
        };
        assert_eq!(transition_for(&cfg, Stage::Review), Some("In Review"));
        assert_eq!(transition_for(&cfg, Stage::Build), None);
        assert_eq!(
            transition_for(&JiraTransitions { enabled: false, ..cfg }, Stage::Review),
            None
        );
    }
}
