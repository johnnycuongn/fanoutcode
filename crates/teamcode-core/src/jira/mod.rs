//! Jira Cloud and Confluence (§12): the REST clients, issue text, the per-project issue cache and the settings keys.

pub mod adf;
pub mod cache;
pub mod client;
pub mod confluence;
pub mod intent;
pub mod settings;
#[cfg(test)]
pub(crate) mod test_support;
