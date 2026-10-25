//! The Jira settings keys: the linked site (workspace default, project override) and the optional stage-to-status
//! transitions (project only).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::client::{normalise_site, JiraSite};
use crate::domain::Stage;
use crate::ids::{ProjectId, WorkspaceId};
use crate::settings::{Scope, ScopeKind, ScopeValues};
use crate::{Error, Result};

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

/// The keychain account of the token linked at a project or workspace: derived, with the id re-validated, never read
/// from a setting.
pub fn account_for(kind: ScopeKind, id: &str) -> Result<String> {
    match kind {
        ScopeKind::Project => Ok(format!("jira:project:{}", ProjectId::parse(id)?)),
        ScopeKind::Workspace => Ok(format!("jira:workspace:{}", WorkspaceId::parse(id)?)),
        _ => Err(Error::Invalid("Jira is linked to a project or a workspace.".into())),
    }
}

/// The site the scope resolves to (project first, then workspace) and the keychain account of the layer it came
/// from.
pub fn resolve_with_account(values: &ScopeValues, scope: &Scope) -> Option<(JiraSite, String)> {
    let layer = |kind: ScopeKind, id: Option<&str>, map: &std::collections::HashMap<String, serde_json::Value>| {
        let id = id?;
        let site: JiraSite = serde_json::from_value(map.get("jira")?.clone()).ok()?;
        Some((site, account_for(kind, id).ok()?))
    };
    layer(
        ScopeKind::Project,
        scope.project.as_ref().map(|p| p.as_str()),
        &values.project,
    )
    .or_else(|| {
        layer(
            ScopeKind::Workspace,
            scope.workspace.as_ref().map(|w| w.as_str()),
            &values.workspace,
        )
    })
}

/// What the keychain holds for a link: the token and the site origin it was verified against.
#[derive(Serialize, Deserialize)]
pub struct Credential {
    pub site: String,
    pub token: String,
}

impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Credential {{ site: {:?}, token: <redacted> }}", self.site)
    }
}

/// The keychain text for a token verified against `site_url`.
pub fn credential_text(site_url: &str, token: &str) -> Result<String> {
    Ok(serde_json::to_string(&Credential {
        site: normalise_site(site_url)?,
        token: token.to_owned(),
    })?)
}

/// The token to send to `site`, only when the stored credential was saved for that same origin. A plain token (no
/// site recorded) or a different origin is refused.
pub fn token_for_site(stored: &str, site: &JiraSite) -> Result<String> {
    let relink = || Error::Precondition("The Jira site changed since it was linked; link Jira again.".into());
    let cred: Credential = serde_json::from_str(stored).map_err(|_| relink())?;
    if normalise_site(&site.site_url)? != cred.site {
        return Err(relink());
    }
    Ok(cred.token)
}

/// `settings_set` may not write the `jira` key: only linking does, so a site cannot be repointed at a stored token.
pub fn check_settable(key: &str) -> Result<()> {
    if key == "jira" {
        return Err(Error::Invalid(
            "Link Jira from the Jira settings; the site can't be set directly.".into(),
        ));
    }
    Ok(())
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
        db::settings::set(
            &conn,
            ScopeKind::Workspace,
            ws.as_str(),
            "jira",
            &json!({"site_url": "https://acme.atlassian.net", "email": "me@acme.com"}),
        )
        .unwrap();
        let scope = Scope {
            workspace: Some(ws.clone()),
            project: Some(fx.api.id.clone()),
            task: None,
        };
        let values = db::settings::load(&conn, &scope).unwrap();
        assert_eq!(
            resolve_with_account(&values, &scope).map(|(_, a)| a),
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
        v.workspace.insert("jira".into(), json!({"site_url": "https://acme.atlassian.net", "email": "me@acme.com", "account": "jira:project:p-victim"}));
        assert_eq!(
            resolve_with_account(&v, &scope).map(|(_, a)| a),
            Some("jira:workspace:ws-1a-1".into()),
            "the account comes from the layer, never from a stray field in the value"
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

    #[test]
    fn a_stored_token_goes_only_to_the_site_it_was_linked_for() {
        let linked = JiraSite {
            site_url: "https://acme.atlassian.net/browse/X-1".into(),
            email: "me@acme.com".into(),
        };
        let stored = credential_text(&linked.site_url, "tok").unwrap();
        assert!(!stored.contains("browse"), "the origin is stored");
        assert_eq!(token_for_site(&stored, &linked).unwrap(), "tok");
        let moved = JiraSite {
            site_url: "https://evil.example".into(),
            ..linked.clone()
        };
        assert!(
            matches!(token_for_site(&stored, &moved), Err(Error::Precondition(m)) if m.contains("link Jira again"))
        );
        assert!(
            token_for_site("plain-old-token", &linked).is_err(),
            "a token without its site is not sent"
        );
        assert!(!format!("{:?}", serde_json::from_str::<Credential>(&stored).unwrap()).contains("tok\""));
    }

    #[test]
    fn accounts_are_derived_and_jira_cannot_be_set_directly() {
        assert_eq!(account_for(ScopeKind::Project, "p-1").unwrap(), "jira:project:p-1");
        assert!(account_for(ScopeKind::Project, "../x").is_err());
        assert!(account_for(ScopeKind::App, "").is_err());
        assert!(check_settable("jira").is_err() && check_settable("jira_transitions").is_ok());
    }

    #[tokio::test]
    async fn link_then_fetch_uses_the_bound_token() {
        use wiremock::matchers::{header, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(path("/rest/api/3/issue/PROJ-1"))
            .and(header("Authorization", "Basic bWVAYWNtZS5jb206dG9r"))
            .respond_with(ResponseTemplate::new(200).set_body_json(crate::jira::test_support::issue_json("PROJ-1")))
            .mount(&server)
            .await;
        let site = crate::jira::test_support::site(&server.uri());
        let stored = credential_text(&site.site_url, "tok").unwrap();
        let token = token_for_site(&stored, &site).unwrap();
        let issue = crate::jira::client::JiraClient::new(&site, &token)
            .unwrap()
            .issue("PROJ-1")
            .await
            .unwrap();
        assert_eq!(issue.key, "PROJ-1");
    }
}
