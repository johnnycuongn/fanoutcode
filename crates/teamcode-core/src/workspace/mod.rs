//! Workspaces (§4.1–4.3): the record and its `<name>.teamcode-workspace.json` mirror, linking (bound roots,
//! `--add-dir`, protected roots, the Bash guard), prompt fragments, context docs and container conversion. The
//! database is the source of truth; the file is a mirror (write-through in app data, explicit save elsewhere).

pub mod context_docs;
pub mod file;
pub mod guard;
pub mod import;
pub(crate) mod import_checks;
pub mod link;
pub mod open;
pub mod policy;
pub(crate) mod rows;
pub mod store;
#[cfg(test)]
pub(crate) mod test_support;
pub mod watcher;
