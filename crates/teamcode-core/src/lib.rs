//! Team Code core: domain types, ids, persistence. Never depends on Tauri.

pub mod db;
pub mod domain;
pub mod error;
pub mod ids;
pub mod memory;
pub mod pipeline;
pub mod recent;
#[cfg(test)]
pub(crate) mod test_support;
pub mod windows;

pub use error::{Error, Result};
