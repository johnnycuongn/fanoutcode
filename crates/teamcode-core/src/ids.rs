//! Ids that may become paths. Every id is `<prefix>-<1..40 of [0-9a-f-]>`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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
}
