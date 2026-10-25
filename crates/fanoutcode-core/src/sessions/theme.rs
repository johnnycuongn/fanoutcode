//! The terminal's colours (§11.2): follow the app's theme, force dark or light, or one bundled palette. The value
//! type of the `terminal_theme` setting (A19); the palettes' colours live in `src/styles.css` as `--term-*`.

use serde::{Deserialize, Serialize};

/// The `terminal_theme` setting. Internally tagged (A1) with a struct variant for the named palette (A17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum TerminalTheme {
    /// Soft White in the light app theme, Graphite in the dark one.
    FollowApp,
    AlwaysDark,
    AlwaysLight,
    Named {
        palette: TerminalPalette,
    },
}

/// The bundled palettes; unit-only, so each serialises as its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum TerminalPalette {
    Graphite,
    SoftWhite,
    OneDark,
    SolarizedLight,
    SolarizedDark,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn terminal_theme_is_internally_tagged_and_round_trips() {
        let named = TerminalTheme::Named {
            palette: TerminalPalette::OneDark,
        };
        assert_eq!(
            serde_json::to_value(named).unwrap(),
            json!({"type": "Named", "palette": "OneDark"})
        );
        assert_eq!(
            serde_json::to_value(TerminalTheme::FollowApp).unwrap(),
            json!({"type": "FollowApp"})
        );
        assert_eq!(
            serde_json::from_value::<TerminalTheme>(json!({"type": "Named", "palette": "OneDark"})).unwrap(),
            named
        );
    }

    #[test]
    fn palette_serialises_as_a_plain_string() {
        assert_eq!(
            serde_json::to_value(TerminalPalette::SolarizedDark).unwrap(),
            json!("SolarizedDark")
        );
    }

    #[test]
    fn terminal_theme_key_is_registered_in_its_a12_slot_with_follow_app_default() {
        use crate::settings::{keys::KEYS, resolve_all, validate, Scope, ScopeKind, ScopeValues};
        let names: Vec<&str> = KEYS.iter().map(|k| k.key).collect();
        let at = names
            .iter()
            .position(|k| *k == "terminal_theme")
            .expect("terminal_theme registered");
        assert_eq!(
            (names[at - 1], names[at + 1]),
            ("cost_overview", "terminal_font_family")
        );
        let r = resolve_all(&ScopeValues::default(), &Scope::app());
        assert_eq!(r.terminal_theme, TerminalTheme::FollowApp);
        assert_eq!(
            serde_json::to_value(&r).unwrap()["terminal_theme"],
            json!({"type": "FollowApp"})
        );
        assert!(validate(
            "terminal_theme",
            ScopeKind::App,
            &json!({"type": "Named", "palette": "OneDark"})
        )
        .is_ok());
        assert!(validate("terminal_theme", ScopeKind::App, &json!("oneDark")).is_err()); // A16: no camelCase, and the tag is required
        assert!(validate("terminal_theme", ScopeKind::Project, &json!({"type": "FollowApp"})).is_err());
        // app level only
    }
}
