//! Team Code core: domain types, ids, persistence. Never depends on Tauri.

pub mod db;
pub mod domain;
pub mod error;
pub mod ids;
pub mod recent;

pub use error::{Error, Result};
