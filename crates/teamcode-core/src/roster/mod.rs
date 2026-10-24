//! The subagent roster (§7.4): agent definitions as markdown files with front matter, unioned across app, workspace
//! and project with name shadowing, and handed to `claude --agents` as JSON (Plan 04).

pub mod shipped;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::memory::store::{self, Loaded};
use crate::{Error, Result};

/// Where a definition came from; a more specific level shadows a less specific one by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
pub enum RosterLevel {
    App,
    Workspace,
    Project,
}

/// One subagent: `claude --agents` takes `description`, `prompt`, and optionally `tools` and `model`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub origin: RosterLevel,
}

/// The definitions at each level, least specific first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RosterScope {
    pub app: Vec<AgentDef>,
    pub workspace: Vec<AgentDef>,
    pub project: Vec<AgentDef>,
}

/// Front matter between the first two `---` lines (`name` defaults to `file_stem`; `description` is required;
/// `tools` is a comma list; `model` optional); the body after the closing `---` is the prompt.
pub fn parse_agent(file_stem: &str, text: &str, origin: RosterLevel) -> Result<AgentDef> {
    let invalid = |why: &str| Error::Invalid(format!("Agent {file_stem}: {why}"));
    let rest = text
        .strip_prefix("---\n")
        .ok_or_else(|| invalid("missing front matter"))?;
    let (front, body) = match rest.split_once("\n---\n") {
        Some(parts) => parts,
        None => rest
            .strip_suffix("\n---")
            .map(|f| (f, ""))
            .ok_or_else(|| invalid("unclosed front matter"))?,
    };
    let mut def = AgentDef {
        name: file_stem.to_owned(),
        description: String::new(),
        tools: Vec::new(),
        model: None,
        prompt: body.trim().to_owned(),
        origin,
    };
    for line in front.lines().filter(|l| !l.trim().is_empty()) {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("a front matter line has no key"))?;
        let value = value.trim();
        match key.trim() {
            "name" if !value.is_empty() => def.name = value.to_owned(),
            "description" => def.description = value.to_owned(),
            "tools" => {
                def.tools = value
                    .split(',')
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            "model" if !value.is_empty() => def.model = Some(value.to_owned()),
            _ => {}
        }
    }
    if def.description.is_empty() {
        return Err(invalid("description is required"));
    }
    Ok(def)
}

/// Every `*.md` in `dir`, sorted by file name. A missing folder is empty; a file that fails to parse is skipped.
pub fn load_dir(dir: &Path, origin: RosterLevel) -> Result<Vec<AgentDef>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md") && p.is_file())
        .collect();
    paths.sort();
    let mut defs = Vec::new();
    for path in paths {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Loaded::Parsed(text) = store::load_text(&path)? {
            if let Ok(def) = parse_agent(&stem, &text, origin) {
                defs.push(def);
            }
        }
    }
    Ok(defs)
}

/// The union of the three levels; a name at a more specific level replaces the earlier one in place, so the order
/// of first appearance is kept.
pub fn resolve(scope: &RosterScope) -> Vec<AgentDef> {
    let mut out: Vec<AgentDef> = Vec::new();
    for def in scope.app.iter().chain(&scope.workspace).chain(&scope.project) {
        match out.iter_mut().find(|d| d.name == def.name) {
            Some(slot) => *slot = def.clone(),
            None => out.push(def.clone()),
        }
    }
    out
}

/// `{"<name>":{"description":…,"prompt":…,"tools":[…]?,"model":…?}}`, the shape `claude --agents` accepts.
pub fn to_agents_json(defs: &[AgentDef]) -> String {
    let mut map = serde_json::Map::new();
    for d in defs {
        let mut agent = serde_json::Map::new();
        agent.insert("description".into(), d.description.clone().into());
        agent.insert("prompt".into(), d.prompt.clone().into());
        if !d.tools.is_empty() {
            agent.insert("tools".into(), d.tools.clone().into());
        }
        if let Some(model) = &d.model {
            agent.insert("model".into(), model.clone().into());
        }
        map.insert(d.name.clone(), agent.into());
    }
    serde_json::Value::Object(map).to_string()
}

/// `<app_config>/agents`.
pub fn agents_dir_app(app_config: &Path) -> PathBuf {
    app_config.join("agents")
}

/// `<memory_root>/agents`.
pub fn agents_dir_project(memory_root: &Path) -> PathBuf {
    memory_root.join("agents")
}

/// Writes each shipped file that is absent (an edited or deleted-and-recreated file is never replaced); returns the
/// paths written.
pub fn install_shipped(app_config: &Path) -> Result<Vec<PathBuf>> {
    let dir = agents_dir_app(app_config);
    let mut written = Vec::new();
    for (file, text) in shipped::SHIPPED {
        let path = dir.join(file);
        if !path.exists() {
            store::write_atomic(&path, text.as_bytes())?;
            written.push(path);
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn parse_agent_reads_front_matter_and_body() {
        let a = parse_agent("frontend", shipped::SHIPPED[0].1, RosterLevel::App).unwrap();
        assert_eq!(a.name, "frontend");
        assert_eq!(a.tools, ["Read", "Grep", "Glob", "Edit", "Write", "MultiEdit", "Bash"]);
        assert!(a.description.starts_with("Builds webview work"));
        assert!(a.prompt.starts_with("You implement the plan steps"));
        assert!(
            matches!(
                parse_agent("x", "---\nname: x\n---\nbody", RosterLevel::App),
                Err(Error::Invalid(_))
            ),
            "description is required"
        );
        let b = parse_agent(
            "stem",
            "---\ndescription: d\nmodel: sonnet\n---\np",
            RosterLevel::Project,
        )
        .unwrap();
        assert_eq!(
            (b.name.as_str(), b.model.as_deref(), b.tools.len()),
            ("stem", Some("sonnet"), 0)
        );
    }

    #[test]
    fn shipped_set_is_the_five_roles() {
        let names: Vec<&str> = shipped::SHIPPED.iter().map(|(f, _)| *f).collect();
        assert_eq!(
            names,
            [
                "frontend.md",
                "backend.md",
                "best-practice-reviewer.md",
                "codebase-consistency-reviewer.md",
                "security-reviewer.md"
            ]
        );
        for (f, text) in shipped::SHIPPED {
            assert_eq!(
                parse_agent(f.trim_end_matches(".md"), text, RosterLevel::App)
                    .unwrap()
                    .name,
                f.trim_end_matches(".md")
            );
        }
    }

    #[test]
    fn resolve_unions_and_shadows_by_name_in_place() {
        let def = |n: &str, d: &str, o| AgentDef {
            name: n.into(),
            description: d.into(),
            tools: vec![],
            model: None,
            prompt: String::new(),
            origin: o,
        };
        let scope = RosterScope {
            app: vec![
                def("frontend", "app", RosterLevel::App),
                def("backend", "app", RosterLevel::App),
            ],
            workspace: vec![
                def("backend", "ws", RosterLevel::Workspace),
                def("data", "ws", RosterLevel::Workspace),
            ],
            project: vec![def("frontend", "proj", RosterLevel::Project)],
        };
        let r = resolve(&scope);
        let got: Vec<(&str, &str)> = r.iter().map(|a| (a.name.as_str(), a.description.as_str())).collect();
        assert_eq!(got, [("frontend", "proj"), ("backend", "ws"), ("data", "ws")]);
    }

    #[test]
    fn agents_json_has_the_claude_shape_and_omits_empty_optionals() {
        let app: Vec<AgentDef> = shipped::SHIPPED
            .iter()
            .map(|(f, t)| parse_agent(f.trim_end_matches(".md"), t, RosterLevel::App).unwrap())
            .collect();
        let defs = resolve(&RosterScope {
            app,
            workspace: vec![],
            project: vec![],
        });
        let v: serde_json::Value = serde_json::from_str(&to_agents_json(&defs)).unwrap();
        assert_eq!(v["frontend"]["tools"][0], "Read");
        assert!(v["frontend"]["description"]
            .as_str()
            .unwrap()
            .starts_with("Builds webview work"));
        assert!(v["frontend"].get("model").is_none());
        assert_eq!(v.as_object().unwrap().len(), 5);
    }

    #[test]
    fn install_shipped_writes_missing_files_only_and_load_dir_reads_them() {
        let dir = tempfile::tempdir().unwrap();
        let written = install_shipped(dir.path()).unwrap();
        assert_eq!(written.len(), 5);
        std::fs::write(
            dir.path().join("agents/frontend.md"),
            "---\ndescription: mine\n---\nedited",
        )
        .unwrap();
        assert!(install_shipped(dir.path()).unwrap().is_empty());
        let defs = load_dir(&agents_dir_app(dir.path()), RosterLevel::App).unwrap();
        assert_eq!(defs.iter().find(|d| d.name == "frontend").unwrap().description, "mine");
        assert!(load_dir(Path::new("/definitely/missing"), RosterLevel::Project)
            .unwrap()
            .is_empty());
    }
}
