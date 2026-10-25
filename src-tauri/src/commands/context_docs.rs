//! Context doc commands (Plan 10): the references listed per project or workspace, and fetching a URL's or a
//! Confluence page's text into `<app_data>/context/<doc-id>.md`.

use tauri::{AppHandle, State};
use teamcode_core::domain::Container;
use teamcode_core::jira::settings::JiraSiteKey;
use teamcode_core::memory::store::write_atomic;
use teamcode_core::settings::{resolve, Scope};
use teamcode_core::workspace::context_docs::{fetch_text, text_cache_path, ContextDoc, ContextDocKind};
use teamcode_core::{db, Error, Result};

use super::parse_container;
use crate::events::{self, ContextDocsChanged};
use crate::observe::log_err;
use crate::secrets;
use crate::state::AppState;

fn announce(app: &AppHandle, container: Container) {
    log_err(
        "announcing the references",
        events::emit(app, ContextDocsChanged { container }),
    );
}

#[tauri::command]
#[specta::specta]
pub fn context_docs_list(state: State<'_, AppState>, container: Container) -> Result<Vec<ContextDoc>> {
    state.context_docs.list(&parse_container(&container)?)
}

/// A file or folder path is canonicalised; links are checked by the store.
#[tauri::command]
#[specta::specta]
pub fn context_docs_add(
    app: AppHandle,
    state: State<'_, AppState>,
    container: Container,
    kind: ContextDocKind,
    target: String,
    label: Option<String>,
) -> Result<ContextDoc> {
    let container = parse_container(&container)?;
    let target = match kind {
        ContextDocKind::File | ContextDocKind::Folder => std::fs::canonicalize(target.trim())
            .map_err(|_| Error::Precondition("That file or folder does not exist.".into()))?
            .to_string_lossy()
            .into_owned(),
        _ => target,
    };
    let doc = state.context_docs.add(&container, kind, &target, label.as_deref())?;
    announce(&app, container);
    Ok(doc)
}

#[tauri::command]
#[specta::specta]
pub fn context_docs_remove(app: AppHandle, state: State<'_, AppState>, container: Container, id: String) -> Result<()> {
    let container = parse_container(&container)?;
    state.context_docs.remove(&container, &id)?;
    announce(&app, container);
    Ok(())
}

/// Fetches the reference's text now, caches it and returns it for a preview. Confluence pages use the Atlassian
/// site linked for Jira at the container's scope and its token from the keychain; the token never leaves the shell.
#[tauri::command]
#[specta::specta]
pub async fn context_docs_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    container: Container,
    id: String,
) -> Result<String> {
    let container = parse_container(&container)?;
    let (owner, doc) = state.context_docs.get_with_container(&id)?;
    if owner != container {
        return Err(Error::Invalid("Unknown reference id.".into()));
    }
    let cache = text_cache_path(&state.data_dir, &doc.id)?;
    let site = if doc.kind == ContextDocKind::Confluence {
        let scope = match &container {
            Container::Project { id } => Scope {
                project: Some(id.clone()),
                ..Default::default()
            },
            Container::Workspace { id } => Scope {
                workspace: Some(id.clone()),
                ..Default::default()
            },
        };
        let values = db::settings::load(&state.db.lock().expect("db lock"), &scope)?;
        resolve::<JiraSiteKey>(&values, &scope)
    } else {
        None
    };
    let token = match &site {
        Some(s) => secrets::read(&s.account)?,
        None => None,
    };
    let atlassian = match (&site, &token) {
        (Some(s), Some(t)) => Some((s, t.expose())),
        _ => None,
    };
    let text = fetch_text(&doc, atlassian, &state.http).await?;
    write_atomic(&cache, text.as_bytes())?;
    state.context_docs.set_fetched(&doc.id, chrono::Utc::now())?;
    announce(&app, container);
    Ok(text)
}
