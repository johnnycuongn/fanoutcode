//! Ids that may become paths. Every id is `<prefix>-<1..40 of [0-9a-f-]>`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// `^[a-z]+-[0-9a-f-]{1,40}$`: one or more lower-case letters, a dash, then 1 to 40 of `[0-9a-f-]`.
/// Checked before any id becomes part of a path.
pub fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    let prefix = bytes.iter().take_while(|b| b.is_ascii_lowercase()).count();
    if prefix == 0 || bytes.get(prefix) != Some(&b'-') {
        return false;
    }
    let rest = &bytes[prefix + 1..];
    (1..=40).contains(&rest.len())
        && rest
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b) || *b == b'-')
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A fresh id: `<prefix>-<unix millis hex>-<process counter hex>`.
pub fn new_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis:x}-{n:x}")
}

/// The label of window `n`: `w-<n>`, as in LiveSpec.
pub fn window_id(n: u64) -> String {
    format!("w-{n}")
}

/// A validated id newtype: `#[serde(transparent)]`, so it crosses IPC as a plain string. Deserialising does not
/// validate; commands re-parse ids from the webview with `parse`.
macro_rules! id_newtype {
    ($(#[$doc:meta])* $name:ident, $prefix:literal, $kind:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, specta::Type)]
        #[serde(transparent)]
        #[specta(transparent)]
        pub struct $name(String);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            /// A fresh id with this type's prefix.
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(new_id($prefix))
            }

            /// `Error::Invalid` unless `s` is a valid id with this type's prefix.
            pub fn parse(s: &str) -> Result<Self> {
                if valid_id(s) && s.split('-').next() == Some($prefix) {
                    Ok(Self(s.to_owned()))
                } else {
                    Err(Error::Invalid(concat!("Unknown ", $kind, " id.").to_owned()))
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_newtype!(ProjectId, "p", "project");
id_newtype!(WorkspaceId, "ws", "workspace");
id_newtype!(TaskId, "t", "task");
id_newtype!(ViewId, "v", "view");
id_newtype!(PaneId, "pane", "pane");
id_newtype!(SessionId, "s", "session");
id_newtype!(
    /// `w-<n>`, as in LiveSpec; the Tauri window label.
    WindowId,
    "w",
    "window"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_id_accepts_every_prefix_and_rejects_paths() {
        for ok in [
            "w-1",
            "p-18f2a3b4c5d-1",
            "ws-0",
            "pane-a",
            "t-a-b-c",
            &format!("s-{}", "f".repeat(40)),
        ] {
            assert!(valid_id(ok), "{ok}");
        }
        for bad in [
            "",
            "w-",
            "-1",
            "W-1",
            "p1",
            "p-A1",
            "p-1/2",
            "p-../x",
            "t-1.json",
            "w- 1",
            "w-1\n",
            "p_1",
            &format!("s-{}", "f".repeat(41)),
        ] {
            assert!(!valid_id(bad), "{bad:?}");
        }
    }

    #[test]
    fn generated_ids_validate_and_are_unique() {
        let a = new_id("t");
        let b = new_id("t");
        assert!(valid_id(&a) && valid_id(&b));
        assert_ne!(a, b);
        assert!(a.starts_with("t-"));
        assert_eq!(window_id(3), "w-3");
    }

    #[test]
    fn ids_parse_only_their_own_prefix() {
        assert!(TaskId::parse("t-1a").is_ok());
        assert!(matches!(TaskId::parse("p-1a"), Err(Error::Invalid(_))));
        assert!(matches!(WindowId::parse("w-../x"), Err(Error::Invalid(_))));
        assert_eq!(
            serde_json::to_string(&TaskId::parse("t-1a").unwrap()).unwrap(),
            "\"t-1a\""
        );
    }
}
