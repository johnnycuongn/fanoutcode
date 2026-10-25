//! Jira Cloud and Confluence (§12): the REST clients, issue text, the per-project issue cache and the settings keys.

pub mod adf;
pub mod client;
pub mod confluence;
#[cfg(test)]
pub(crate) mod test_support;
