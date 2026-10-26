//! A test command suggested by the project's own files when none is set (§6.5). It is only ever a suggestion: running
//! it runs the repository's own scripts, so the developer accepts it (Settings, or the question at Approve changes),
//! which saves it as `test_command`; nothing runs a detected command on its own.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A command found in the project's own files, and the file that suggested it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct DetectedTest {
    pub command: String,
    pub source: String,
}

fn found(command: &str, source: &str) -> Option<DetectedTest> {
    Some(DetectedTest {
        command: command.to_owned(),
        source: source.to_owned(),
    })
}

fn read(root: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(root.join(name)).ok()
}

/// `scripts.test` of package.json, unless it is npm's placeholder, run with the package manager the lockfile names.
fn node(root: &Path) -> Option<DetectedTest> {
    let pkg: serde_json::Value = serde_json::from_str(&read(root, "package.json")?).ok()?;
    let script = pkg.get("scripts")?.get("test")?.as_str()?.trim();
    if script.is_empty() || script.contains("no test specified") {
        return None;
    }
    let runner = if root.join("pnpm-lock.yaml").exists() {
        "pnpm test"
    } else if root.join("yarn.lock").exists() {
        "yarn test"
    } else if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        "bun run test"
    } else {
        "npm test"
    };
    found(runner, "package.json")
}

fn pytest(root: &Path) -> Option<DetectedTest> {
    if root.join("pytest.ini").exists() {
        return found("pytest", "pytest.ini");
    }
    if read(root, "pyproject.toml").is_some_and(|t| t.contains("[tool.pytest")) {
        return found("pytest", "pyproject.toml");
    }
    if read(root, "setup.cfg").is_some_and(|t| t.contains("[tool:pytest]")) {
        return found("pytest", "setup.cfg");
    }
    None
}

fn make(root: &Path) -> Option<DetectedTest> {
    let text = read(root, "Makefile")?;
    text.lines()
        .any(|l| l.starts_with("test:"))
        .then(|| found("make test", "Makefile"))?
}

/// The first of: package.json's test script, Cargo.toml, go.mod, a pytest config, a Makefile `test` target.
pub fn detect(root: &Path) -> Option<DetectedTest> {
    node(root)
        .or_else(|| {
            root.join("Cargo.toml")
                .exists()
                .then(|| found("cargo test", "Cargo.toml"))?
        })
        .or_else(|| root.join("go.mod").exists().then(|| found("go test ./...", "go.mod"))?)
        .or_else(|| pytest(root))
        .or_else(|| make(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            std::fs::write(dir.path().join(name), text).unwrap();
        }
        dir
    }

    #[test]
    fn a_package_test_script_runs_with_the_lockfiles_package_manager() {
        let npm = dir_with(&[("package.json", r#"{"scripts":{"test":"node --test test/"}}"#)]);
        assert_eq!(detect(npm.path()), found("npm test", "package.json"));
        let pnpm = dir_with(&[
            ("package.json", r#"{"scripts":{"test":"vitest run"}}"#),
            ("pnpm-lock.yaml", ""),
        ]);
        assert_eq!(detect(pnpm.path()).unwrap().command, "pnpm test");
    }

    #[test]
    fn npms_placeholder_script_is_no_test_command() {
        let d = dir_with(&[(
            "package.json",
            r#"{"scripts":{"test":"echo \"Error: no test specified\" && exit 1"}}"#,
        )]);
        assert_eq!(detect(d.path()), None);
    }

    #[test]
    fn other_ecosystems_are_found_and_a_plain_folder_has_none() {
        assert_eq!(
            detect(dir_with(&[("Cargo.toml", "")]).path()).unwrap().command,
            "cargo test"
        );
        assert_eq!(
            detect(dir_with(&[("go.mod", "")]).path()).unwrap().command,
            "go test ./..."
        );
        assert_eq!(
            detect(dir_with(&[("pyproject.toml", "[tool.pytest.ini_options]\n")]).path()),
            found("pytest", "pyproject.toml")
        );
        assert_eq!(
            detect(dir_with(&[("Makefile", "build:\n\tcc x.c\ntest:\n\t./t\n")]).path())
                .unwrap()
                .command,
            "make test"
        );
        assert_eq!(detect(tempfile::tempdir().unwrap().path()), None);
    }
}
