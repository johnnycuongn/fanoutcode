//! Team Code core: domain types, ids, persistence. Never depends on Tauri.

pub mod changes;
pub mod cost;
pub mod db;
pub mod domain;
pub mod error;
pub mod git;
pub mod helpers;
pub mod hooks;
pub mod ids;
pub mod memory;
pub mod pipeline;
pub mod prompts;
pub mod recent;
pub mod roster;
pub mod settings;
#[cfg(test)]
pub(crate) mod test_support;
pub mod testrun;
pub mod views;
pub mod windows;
pub mod worktree;

pub use error::{Error, Result};
