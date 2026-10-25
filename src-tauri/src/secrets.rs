//! Secrets live in the OS keychain under service `app.fanoutcode.desktop` (A9), never in SQLite or the webview.
//! Nothing here crosses IPC: `Secret` has no `Serialize`, `Deserialize` or `specta::Type`.
//!
//! Accounts are `[a-z0-9._:-]{1,128}`, by convention `<domain>:<id>` (e.g. `jira:p-18f2a3b4c5d-1`).

use fanoutcode_core::{Error, Result};

pub const SERVICE: &str = "app.fanoutcode.desktop";

/// A secret value. `Debug` prints `Secret(***)`; read the value only with `expose`, at the point of use.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(***)")
    }
}

/// The testing seam: the OS keychain in the app, a map in tests.
pub(crate) trait Keychain {
    fn get(&self, account: &str) -> Result<Option<String>>;
    fn set(&self, account: &str, value: &str) -> Result<()>;
    fn remove(&self, account: &str) -> Result<()>;
}

/// The macOS Keychain (or the platform's store) through the `keyring` crate.
struct OsKeychain;

fn keychain_error(e: keyring::Error) -> Error {
    // keyring's messages describe the failure, never the stored value.
    Error::Io(format!("Keychain: {e}"))
}

impl Keychain for OsKeychain {
    fn get(&self, account: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(keychain_error)?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keychain_error(e)),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<()> {
        keyring::Entry::new(SERVICE, account)
            .and_then(|e| e.set_password(value))
            .map_err(keychain_error)
    }

    fn remove(&self, account: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, account).map_err(keychain_error)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(keychain_error(e)),
        }
    }
}

fn check_account(account: &str) -> Result<()> {
    let ok = (1..=128).contains(&account.len())
        && account
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".:_-".contains(&b));
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid("Bad keychain account name.".into()))
    }
}

pub(crate) fn read_with(k: &impl Keychain, account: &str) -> Result<Option<Secret>> {
    check_account(account)?;
    Ok(k.get(account)?.map(Secret::new))
}

pub(crate) fn write_with(k: &impl Keychain, account: &str, value: &str) -> Result<()> {
    check_account(account)?;
    k.set(account, value)
}

pub(crate) fn delete_with(k: &impl Keychain, account: &str) -> Result<()> {
    check_account(account)?;
    k.remove(account)
}

/// `Ok(None)` when no item exists for `account`.
pub fn read(account: &str) -> Result<Option<Secret>> {
    read_with(&OsKeychain, account)
}

/// Creates or replaces the item for `account`.
pub fn write(account: &str, value: &str) -> Result<()> {
    write_with(&OsKeychain, account, value)
}

/// Deleting a missing item is `Ok(())`.
pub fn delete(account: &str) -> Result<()> {
    delete_with(&OsKeychain, account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemoryKeychain(RefCell<HashMap<String, String>>);

    impl Keychain for MemoryKeychain {
        fn get(&self, account: &str) -> Result<Option<String>> {
            Ok(self.0.borrow().get(account).cloned())
        }
        fn set(&self, account: &str, value: &str) -> Result<()> {
            self.0.borrow_mut().insert(account.into(), value.into());
            Ok(())
        }
        fn remove(&self, account: &str) -> Result<()> {
            self.0.borrow_mut().remove(account);
            Ok(())
        }
    }

    #[test]
    fn secret_debug_never_prints_the_value() {
        let s = Secret::new("sk-live-123".into());
        assert_eq!(format!("{s:?}"), "Secret(***)");
        assert_eq!(format!("{:?}", Some(&s)), "Some(Secret(***))");
        assert_eq!(s.expose(), "sk-live-123");
    }

    #[test]
    fn write_read_delete_round_trip_and_missing_is_none() {
        let k = MemoryKeychain::default();
        assert!(read_with(&k, "jira:p-1").unwrap().is_none());
        write_with(&k, "jira:p-1", "tok").unwrap();
        write_with(&k, "jira:p-1", "tok2").unwrap();
        assert_eq!(read_with(&k, "jira:p-1").unwrap().unwrap().expose(), "tok2");
        delete_with(&k, "jira:p-1").unwrap();
        delete_with(&k, "jira:p-1").unwrap();
        assert!(read_with(&k, "jira:p-1").unwrap().is_none());
    }

    #[test]
    fn bad_account_names_are_rejected_before_the_keychain_is_touched() {
        let k = MemoryKeychain::default();
        for bad in ["", "Jira:p-1", "jira p-1", "../x", "a\n", &"a".repeat(129)] {
            assert!(matches!(read_with(&k, bad), Err(Error::Invalid(_))), "{bad:?}");
            assert!(matches!(write_with(&k, bad, "v"), Err(Error::Invalid(_))), "{bad:?}");
        }
        assert!(k.0.borrow().is_empty());
    }

    #[test]
    #[ignore = "touches the real macOS keychain; run by hand"]
    fn os_keychain_round_trip() {
        let account = format!("test:{}", fanoutcode_core::ids::new_id("t"));
        write(&account, "v").unwrap();
        assert_eq!(read(&account).unwrap().unwrap().expose(), "v");
        delete(&account).unwrap();
        assert!(read(&account).unwrap().is_none());
    }
}
