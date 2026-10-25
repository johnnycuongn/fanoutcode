//! What a session runs: the launch spec, and (Tasks 6 and 7) the generated settings file and the per-kind builders.

use std::path::PathBuf;

/// A program, its arguments, its working directory and the extra environment it gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}
