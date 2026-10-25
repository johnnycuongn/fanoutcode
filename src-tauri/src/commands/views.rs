//! View commands. Every mutation validates the new layout, saves it, and sends `views-changed` for the view's
//! container to every window.

use fanoutcode_core::domain::{Container, LayoutNode, PaneContent, PaneId, SplitDirection, View, ViewId};
use fanoutcode_core::views::layout::{self, Branch};
use fanoutcode_core::views::new_view;
use fanoutcode_core::{db, Error, Result};
use tauri::{AppHandle, State};

use super::parse_container;
use crate::events::{self, ViewsChanged};
use crate::state::AppState;

fn view_name(name: &str) -> Result<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("View name cannot be empty".into()));
    }
    Ok(name)
}

fn changed(app: &AppHandle, container: Container) -> Result<()> {
    events::emit(app, ViewsChanged { container })
}

fn unknown_pane() -> Error {
    Error::Invalid("Unknown pane".into())
}

/// Applies `edit` to the view's layout, validates and saves the result, and notifies.
fn edit_layout(
    app: &AppHandle,
    state: &AppState,
    id: &ViewId,
    edit: impl FnOnce(&LayoutNode) -> Option<LayoutNode>,
) -> Result<View> {
    let id = ViewId::parse(id.as_str())?;
    let view = {
        let conn = state.db.lock().expect("db lock");
        let mut view = db::views::require(&conn, &id)?;
        let layout = edit(&view.layout).ok_or_else(unknown_pane)?;
        layout::validate(&layout)?;
        db::views::update_layout(&conn, &id, &layout)?;
        view.layout = layout;
        view
    };
    changed(app, view.container.clone())?;
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn views_list(state: State<'_, AppState>, container: Container) -> Result<Vec<View>> {
    db::views::list(&state.db.lock().expect("db lock"), &parse_container(&container)?)
}

#[tauri::command]
#[specta::specta]
pub fn views_create(app: AppHandle, state: State<'_, AppState>, container: Container, name: String) -> Result<View> {
    let container = parse_container(&container)?;
    let view = {
        let conn = state.db.lock().expect("db lock");
        let view = new_view(
            container.clone(),
            view_name(&name)?,
            db::views::next_order(&conn, &container)?,
        );
        db::views::insert(&conn, &view)?;
        view
    };
    changed(&app, container)?;
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn views_update_layout(app: AppHandle, state: State<'_, AppState>, id: ViewId, layout: LayoutNode) -> Result<View> {
    edit_layout(&app, &state, &id, |_| Some(layout))
}

#[tauri::command]
#[specta::specta]
pub fn views_rename(app: AppHandle, state: State<'_, AppState>, id: ViewId, name: String) -> Result<View> {
    let id = ViewId::parse(id.as_str())?;
    let view = {
        let conn = state.db.lock().expect("db lock");
        db::views::rename(&conn, &id, view_name(&name)?)?;
        db::views::require(&conn, &id)?
    };
    changed(&app, view.container.clone())?;
    Ok(view)
}

#[tauri::command]
#[specta::specta]
pub fn views_reorder(app: AppHandle, state: State<'_, AppState>, container: Container, ids: Vec<ViewId>) -> Result<()> {
    let container = parse_container(&container)?;
    let ids = ids
        .iter()
        .map(|id| ViewId::parse(id.as_str()))
        .collect::<Result<Vec<_>>>()?;
    db::views::reorder(&state.db.lock().expect("db lock"), &container, &ids)?;
    changed(&app, container)
}

#[tauri::command]
#[specta::specta]
pub fn views_delete(app: AppHandle, state: State<'_, AppState>, id: ViewId) -> Result<()> {
    let id = ViewId::parse(id.as_str())?;
    let container = {
        let conn = state.db.lock().expect("db lock");
        let view = db::views::require(&conn, &id)?;
        db::views::delete(&conn, &id)?;
        view.container
    };
    changed(&app, container)
}

/// Splits `pane`, putting a new empty pane second (`Horizontal` side by side, `Vertical` stacked).
#[tauri::command]
#[specta::specta]
pub fn views_split(
    app: AppHandle,
    state: State<'_, AppState>,
    id: ViewId,
    pane: PaneId,
    direction: SplitDirection,
) -> Result<View> {
    edit_layout(&app, &state, &id, |l| layout::split(l, &pane, direction, PaneId::new()))
}

#[tauri::command]
#[specta::specta]
pub fn views_close_pane(app: AppHandle, state: State<'_, AppState>, id: ViewId, pane: PaneId) -> Result<View> {
    edit_layout(&app, &state, &id, |l| layout::close(l, &pane))
}

#[tauri::command]
#[specta::specta]
pub fn views_move_pane(
    app: AppHandle,
    state: State<'_, AppState>,
    id: ViewId,
    from: PaneId,
    to: PaneId,
) -> Result<View> {
    edit_layout(&app, &state, &id, |l| layout::move_pane(l, &from, &to))
}

#[tauri::command]
#[specta::specta]
pub fn views_set_ratio(
    app: AppHandle,
    state: State<'_, AppState>,
    id: ViewId,
    path: Vec<Branch>,
    ratio: f32,
) -> Result<View> {
    edit_layout(&app, &state, &id, |l| layout::set_ratio(l, &path, ratio))
}

#[tauri::command]
#[specta::specta]
pub fn views_set_pane(
    app: AppHandle,
    state: State<'_, AppState>,
    id: ViewId,
    pane: PaneId,
    content: PaneContent,
) -> Result<View> {
    edit_layout(&app, &state, &id, |l| layout::set_content(l, &pane, content))
}
