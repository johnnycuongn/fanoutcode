//! Jira commands (Plan 10): linking a site (token to the keychain, never back across IPC), issues, search,
//! comments, creating a task from an issue, and transitions. The client and the rules live in `fanoutcode_core::jira`.

use fanoutcode_core::domain::{ProjectId, Task, WorkspaceId};
use fanoutcode_core::jira::cache::CachedIssue;
use fanoutcode_core::jira::client::{normalise_site, search_jql, Issue, JiraClient, JiraSite, Transition};
use fanoutcode_core::jira::intent::intent_from_issue;
use fanoutcode_core::jira::settings::{account_for, credential_text, resolve_with_account, token_for_site};
use fanoutcode_core::settings::{Scope, ScopeKind};
use fanoutcode_core::workspace::url_guard::check_lexical;
use fanoutcode_core::{db, Error, Result};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use super::parse_scope;
use crate::secrets;
use crate::state::AppState;

/// The site a project uses and whether it comes from a workspace rather than the project itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct JiraLinkStatus {
    pub site: Option<JiraSite>,
    pub inherited: bool,
}

fn pid(id: &ProjectId) -> Result<ProjectId> {
    ProjectId::parse(id.as_str())
}

/// The project's scope for the `jira` key: itself, and the first workspace holding it that links a site.
pub(crate) fn project_scope(state: &AppState, project: &ProjectId) -> Result<Scope> {
    let conn = state.db.lock().expect("db lock");
    let mut stmt = conn.prepare(
        "SELECT wp.workspace_id FROM workspace_projects wp JOIN settings s
           ON s.scope_kind = 'workspace' AND s.scope_id = wp.workspace_id AND s.key = 'jira'
         WHERE wp.project_id = ?1 ORDER BY wp.workspace_id LIMIT 1",
    )?;
    let workspace: Option<String> = stmt.query_map([project.as_str()], |r| r.get(0))?.next().transpose()?;
    Ok(Scope {
        workspace: workspace.map(|w| WorkspaceId::parse(&w)).transpose()?,
        project: Some(project.clone()),
        task: None,
    })
}

fn resolved_site(state: &AppState, project: &ProjectId) -> Result<Option<(JiraSite, String)>> {
    let scope = project_scope(state, project)?;
    let values = db::settings::load(&state.db.lock().expect("db lock"), &scope)?;
    Ok(resolve_with_account(&values, &scope))
}

/// The token for `site` from `account`, only when it was linked for that site's origin.
pub(crate) fn bound_token(site: &JiraSite, account: &str) -> Result<String> {
    let stored = secrets::read(account)?
        .ok_or_else(|| Error::Precondition("Jira token missing from the keychain; link again".into()))?;
    token_for_site(stored.expose(), site)
}

/// The client for the project's linked site, with the token linked for that site.
pub(crate) fn client_for(state: &AppState, project: &ProjectId) -> Result<JiraClient> {
    let (site, account) = resolved_site(state, project)?
        .ok_or_else(|| Error::Precondition("No Jira site linked for this project".into()))?;
    JiraClient::new(&site, &bound_token(&site, &account)?)
}

/// Checks the credentials, stores the token bound to the site's origin under the account derived from `(kind, id)`,
/// and saves the site there.
async fn link(state: &AppState, kind: ScopeKind, id: &str, site: &str, email: &str, token: &str) -> Result<()> {
    let account = account_for(kind, id)?;
    let site_url = normalise_site(site)?;
    check_lexical(&site_url, true)?;
    let email = email.trim();
    if email.is_empty() || token.trim().is_empty() {
        return Err(Error::Invalid("Enter the account email and an API token.".into()));
    }
    let linked = JiraSite {
        site_url,
        email: email.to_owned(),
    };
    JiraClient::new(&linked, token.trim())?.verify().await?;
    secrets::write(&account, &credential_text(&linked.site_url, token.trim())?)?;
    db::settings::set(
        &state.db.lock().expect("db lock"),
        kind,
        id,
        "jira",
        &serde_json::to_value(&linked)?,
    )
}

#[tauri::command]
#[specta::specta]
pub async fn jira_link(
    state: State<'_, AppState>,
    project_id: ProjectId,
    site: String,
    email: String,
    token: String,
) -> Result<()> {
    let id = pid(&project_id)?;
    link(&state, ScopeKind::Project, id.as_str(), &site, &email, &token).await
}

/// The workspace's default site, inherited by its projects without their own.
#[tauri::command]
#[specta::specta]
pub async fn jira_link_workspace(
    state: State<'_, AppState>,
    workspace_id: WorkspaceId,
    site: String,
    email: String,
    token: String,
) -> Result<()> {
    let id = WorkspaceId::parse(workspace_id.as_str())?;
    link(&state, ScopeKind::Workspace, id.as_str(), &site, &email, &token).await
}

/// Removes the site at the scope's most specific layer and its token.
#[tauri::command]
#[specta::specta]
pub fn jira_unlink(state: State<'_, AppState>, scope: Scope) -> Result<()> {
    let (kind, id) = parse_scope(&scope)?.most_specific();
    if !matches!(kind, ScopeKind::Project | ScopeKind::Workspace) {
        return Err(Error::Invalid("Unlink a project or a workspace.".into()));
    }
    secrets::delete(&account_for(kind, &id)?)?;
    db::settings::set(
        &state.db.lock().expect("db lock"),
        kind,
        &id,
        "jira",
        &serde_json::Value::Null,
    )
}

#[tauri::command]
#[specta::specta]
pub fn jira_status(state: State<'_, AppState>, project_id: ProjectId) -> Result<JiraLinkStatus> {
    let id = pid(&project_id)?;
    let own = db::settings::get_raw(
        &state.db.lock().expect("db lock"),
        ScopeKind::Project,
        id.as_str(),
        "jira",
    )?;
    let site = resolved_site(&state, &id)?.map(|(s, _)| s);
    Ok(JiraLinkStatus {
        inherited: own.is_none() && site.is_some(),
        site,
    })
}

/// A live fetch, cached for the project.
#[tauri::command]
#[specta::specta]
pub async fn jira_issue(state: State<'_, AppState>, project_id: ProjectId, key: String) -> Result<Issue> {
    let id = pid(&project_id)?;
    let issue = client_for(&state, &id)?.issue(key.trim()).await?;
    state.issue_cache.upsert(&id, &issue)?;
    Ok(issue)
}

#[tauri::command]
#[specta::specta]
pub fn jira_cached(state: State<'_, AppState>, project_id: ProjectId) -> Result<Vec<CachedIssue>> {
    state.issue_cache.for_project(&pid(&project_id)?)
}

#[tauri::command]
#[specta::specta]
pub async fn jira_search(state: State<'_, AppState>, project_id: ProjectId, query: String) -> Result<Vec<Issue>> {
    let id = pid(&project_id)?;
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    client_for(&state, &id)?.search(&search_jql(&query), 20).await
}

#[tauri::command]
#[specta::specta]
pub async fn jira_comment(state: State<'_, AppState>, project_id: ProjectId, key: String, body: String) -> Result<()> {
    client_for(&state, &pid(&project_id)?)?.comment(key.trim(), &body).await
}

/// A task from the issue: its intent written from the issue, its Jira key set, its title the summary.
#[tauri::command]
#[specta::specta]
pub async fn jira_create_task(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: ProjectId,
    key: String,
    workspace_id: Option<WorkspaceId>,
) -> Result<Task> {
    let id = pid(&project_id)?;
    let issue = client_for(&state, &id)?.issue(key.trim()).await?;
    state.issue_cache.upsert(&id, &issue)?;
    let task = super::tasks::tasks_create(
        app.clone(),
        state.clone(),
        id,
        intent_from_issue(&issue),
        Some(issue.key.clone()),
        workspace_id,
    )?;
    super::tasks::tasks_rename(app, state, task.id, issue.summary)
}

#[tauri::command]
#[specta::specta]
pub async fn jira_transitions(
    state: State<'_, AppState>,
    project_id: ProjectId,
    key: String,
) -> Result<Vec<Transition>> {
    client_for(&state, &pid(&project_id)?)?.transitions(key.trim()).await
}

#[tauri::command]
#[specta::specta]
pub async fn jira_transition(
    state: State<'_, AppState>,
    project_id: ProjectId,
    key: String,
    transition_id: String,
) -> Result<()> {
    if transition_id.is_empty() || !transition_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(Error::Invalid("Unknown transition.".into()));
    }
    client_for(&state, &pid(&project_id)?)?
        .transition(key.trim(), &transition_id)
        .await
}
