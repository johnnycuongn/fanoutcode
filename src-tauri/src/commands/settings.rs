//! Settings commands: first-run state (§4.4) and scoped settings (§6.8, §13).

use fanoutcode_core::agent_check::{self, AgentCli};
use fanoutcode_core::db;
use fanoutcode_core::domain::{ThemePreference, WorktreeConvention};
use fanoutcode_core::settings::{self, ResolvedSettings, Scope};
use fanoutcode_core::Result;
use tauri::State;

use super::parse_scope;
use crate::state::AppState;

/// The agent CLIs first run looks for (Claude Code, Codex): found on the login shell's PATH, with their versions.
#[tauri::command]
#[specta::specta]
pub async fn agents_check() -> Result<Vec<AgentCli>> {
    let path = livespec_core::shell_env::login_shell_path()
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();
    Ok(agent_check::check(&path).await)
}

/// True until onboarding has been completed; a database that cannot be read counts as pending.
#[tauri::command]
#[specta::specta]
pub fn settings_first_run_pending(state: State<'_, AppState>) -> bool {
    db::first_run_pending(&state.db.lock().expect("db lock")).unwrap_or(true)
}

/// Records the two first-run answers and marks first run complete.
#[tauri::command]
#[specta::specta]
pub fn settings_first_run_complete(
    state: State<'_, AppState>,
    convention: WorktreeConvention,
    theme: ThemePreference,
) -> Result<()> {
    db::complete_first_run(&state.db.lock().expect("db lock"), convention, theme)
}

/// Every key resolved at `scope`, most specific layer winning.
#[tauri::command]
#[specta::specta]
pub fn settings_get(state: State<'_, AppState>, scope: Scope) -> Result<ResolvedSettings> {
    let scope = parse_scope(&scope)?;
    let values = db::settings::load(&state.db.lock().expect("db lock"), &scope)?;
    Ok(settings::resolve_all(&values, &scope))
}

/// Stores `value` for `key` at the scope's most specific layer; `null` removes that layer's override. An unknown
/// key, a level the key does not allow, or a value of the wrong type is `Invalid` and nothing is stored.
#[tauri::command]
#[specta::specta]
pub fn settings_set(state: State<'_, AppState>, scope: Scope, key: String, value: serde_json::Value) -> Result<()> {
    let (kind, id) = parse_scope(&scope)?.most_specific();
    // only linking writes the Jira site, so a setting can't repoint a stored token
    fanoutcode_core::jira::settings::check_settable(&key)?;
    settings::validate(&key, kind, &value)?;
    db::settings::set(&state.db.lock().expect("db lock"), kind, &id, &key, &value)
}
