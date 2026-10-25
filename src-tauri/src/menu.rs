//! The native menu, built explicitly so it claims no chord of the app's keymap (spec §10.5, `src/lib/keyboard.ts`).
//! Without it Tauri installs `Menu::default()`, whose Close Window takes ⌘W before the webview sees it (⌘W closes the
//! focused pane). AppKit resolves menu key equivalents first, so every accelerator here must stay off the keymap;
//! the test below reads the keymap and fails on any overlap.

use tauri::menu::{
    AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu, HELP_SUBMENU_ID, WINDOW_SUBMENU_ID,
};
use tauri::{AppHandle, Manager, Runtime};

/// The menu id of Close Window.
pub const CLOSE_WINDOW: &str = "close-window";

/// Close Window's accelerator: ⌘⇧W, since ⌘W closes the focused pane.
pub const CLOSE_WINDOW_ACCELERATOR: &str = "CmdOrCtrl+Shift+W";

/// Every chord this menu claims, as keymap labels (modifier glyphs then the key). The predefined items carry
/// AppKit's fixed equivalents; keep this list in step with `build`.
pub const MENU_CHORDS: &[&str] = &[
    "⌘H",  // Hide
    "⌘⌥H", // Hide Others
    "⌘Q",  // Quit
    "⌘Z",  // Undo
    "⌘⇧Z", // Redo
    "⌘X",  // Cut
    "⌘C",  // Copy (the terminal keeps it: TERMINAL_RESERVED)
    "⌘V",  // Paste
    "⌘A",  // Select All
    "⌘⌃F", // Enter Full Screen
    "⌘M",  // Minimize
    "⌘⇧W", // Close Window (CLOSE_WINDOW_ACCELERATOR)
];

/// App, Edit, View, Window and Help. No Close Window on ⌘W and no File menu (whose default is Close Window ⌘W).
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let info = app.package_info();
    let about = AboutMetadata {
        name: Some(info.name.clone()),
        version: Some(info.version.to_string()),
        ..Default::default()
    };
    let sep = || PredefinedMenuItem::separator(app);
    let app_menu = Submenu::with_items(
        app,
        info.name.clone(),
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(about))?,
            &sep()?,
            &PredefinedMenuItem::services(app, None)?,
            &sep()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &sep()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &sep()?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let view = Submenu::with_items(app, "View", true, &[&PredefinedMenuItem::fullscreen(app, None)?])?;
    let close = MenuItem::with_id(app, CLOSE_WINDOW, "Close Window", true, Some(CLOSE_WINDOW_ACCELERATOR))?;
    let window = Submenu::with_id_and_items(
        app,
        WINDOW_SUBMENU_ID,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &sep()?,
            &close,
            &sep()?,
            &PredefinedMenuItem::bring_all_to_front(app, None)?,
        ],
    )?;
    let help = Submenu::with_id_and_items(app, HELP_SUBMENU_ID, "Help", true, &[])?;
    Menu::with_items(app, &[&app_menu, &edit, &view, &window, &help])
}

/// Registered with `Builder::on_menu_event`: Close Window closes the focused window (its `CloseRequested` runs).
pub fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    if event.id() != CLOSE_WINDOW {
        return;
    }
    let focused = app
        .webview_windows()
        .into_values()
        .find(|w| w.is_focused().unwrap_or(false));
    if let Some(w) = focused {
        if let Err(e) = w.close() {
            eprintln!("Team Code: couldn't close the window: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A chord label as (sorted modifiers, lower-case key): "⌘⇧[" and "⇧⌘[" compare equal.
    fn norm(label: &str) -> (BTreeSet<char>, String) {
        let mods: BTreeSet<char> = label.chars().filter(|c| "⌘⇧⌥⌃".contains(*c)).collect();
        let key: String = label.chars().filter(|c| !"⌘⇧⌥⌃".contains(*c)).collect();
        (mods, key.to_lowercase())
    }

    /// Every chord label in `src/lib/keyboard.ts`: the quoted strings that start with a modifier glyph.
    fn keymap_chords() -> Vec<String> {
        let ts = include_str!("../../src/lib/keyboard.ts");
        let keymap = &ts[ts.find("const DIGITS").expect("DIGITS")..ts.find("TERMINAL_RESERVED").expect("reserved")];
        keymap
            .split('"')
            .skip(1)
            .step_by(2)
            .filter(|s| s.starts_with(['⌘', '⌃']))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn the_menu_claims_no_chord_of_the_keymap() {
        let keymap = keymap_chords();
        assert!(keymap.len() > 20 && keymap.iter().any(|c| c == "⌘W"), "{keymap:?}");
        let claimed: Vec<_> = keymap.iter().map(|c| norm(c)).collect();
        for chord in MENU_CHORDS {
            assert!(
                !claimed.contains(&norm(chord)),
                "the menu claims {chord}, which the keymap uses"
            );
        }
    }

    #[test]
    fn close_window_is_command_shift_w() {
        assert_eq!(CLOSE_WINDOW_ACCELERATOR, "CmdOrCtrl+Shift+W");
        assert!(MENU_CHORDS.contains(&"⌘⇧W") && !MENU_CHORDS.contains(&"⌘W"));
    }
}
