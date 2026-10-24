//! Which subagent did what (§10.7): `SubagentStart` carries only an id and a type, so its dispatch (and with it the
//! step numbers) is found through Claude Code's `subagents/agent-<id>.meta.json`, the `tool_use` ids in the
//! subagent transcripts, or, failing both, dispatch order.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::parse::normalize_agent_id;
use super::plan_steps::Dispatch;

/// What Claude Code's `agent-<id>.meta.json` says about a subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMeta {
    pub agent_id: String,
    pub agent_type: String,
    pub description: Option<String>,
    pub tool_use_id: Option<String>,
}

/// `<dir>/<transcript stem>/subagents`, where Claude Code keeps a session's subagent transcripts.
pub fn subagent_dir(transcript_path: &Path) -> PathBuf {
    let stem = transcript_path.file_stem().unwrap_or_default();
    transcript_path.with_file_name(stem).join("subagents")
}

/// Reads `agent-<id>.meta.json` (`agentType`, `description`, `toolUseId`); `None` when missing or unreadable.
pub fn read_agent_meta(transcript_path: &Path, agent_id: &str) -> Option<AgentMeta> {
    let id = normalize_agent_id(agent_id);
    let path = subagent_dir(transcript_path).join(format!("agent-{id}.meta.json"));
    let v: Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
    Some(AgentMeta {
        agent_id: id,
        agent_type: text("agentType").unwrap_or_default(),
        description: text("description"),
        tool_use_id: text("toolUseId"),
    })
}

/// Every `tool_use` block id in a session's subagent transcripts, mapped to the subagent that made it.
#[derive(Debug, Clone, Default)]
pub struct ToolUseIndex {
    by_tool_use: HashMap<String, String>,
}

impl ToolUseIndex {
    /// Reads each `agent-*.jsonl`, tolerating unparsable lines; ids are keyed by the file's `agentId`, else its name.
    pub fn scan(subagent_dir: &Path) -> ToolUseIndex {
        let mut idx = ToolUseIndex::default();
        let Ok(entries) = std::fs::read_dir(subagent_dir) else {
            return idx;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(stem) = name.strip_suffix(".jsonl").filter(|n| n.starts_with("agent-")) else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let mut agent = normalize_agent_id(stem);
            for line in text.lines() {
                let Ok(v) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                if let Some(id) = v.get("agentId").and_then(Value::as_str) {
                    agent = normalize_agent_id(id);
                }
                let blocks = v.pointer("/message/content").and_then(Value::as_array);
                for b in blocks.into_iter().flatten() {
                    if b.get("type").and_then(Value::as_str) == Some("tool_use") {
                        if let Some(id) = b.get("id").and_then(Value::as_str) {
                            idx.by_tool_use.insert(id.to_owned(), agent.clone());
                        }
                    }
                }
            }
        }
        idx
    }

    pub fn agent_for(&self, tool_use_id: &str) -> Option<&str> {
        self.by_tool_use.get(tool_use_id).map(String::as_str)
    }
}

/// Matches each `SubagentStart` with the `Task`/`Agent` dispatch that started it.
#[derive(Debug, Clone, Default)]
pub struct Correlator {
    unmatched: VecDeque<(String, Dispatch)>,
    by_agent: HashMap<String, Dispatch>,
}

impl Correlator {
    /// A `PreToolUse` of `Task` or `Agent`, keyed by its `tool_use_id`.
    pub fn note_dispatch(&mut self, tool_use_id: &str, d: Dispatch) {
        self.unmatched.push_back((tool_use_id.to_owned(), d));
    }

    /// The dispatch of a starting subagent: by the meta's `tool_use_id`, else the oldest unmatched one.
    pub fn on_subagent_start(&mut self, agent_id: &str, meta: Option<&AgentMeta>) -> Option<Dispatch> {
        let by_id = meta
            .and_then(|m| m.tool_use_id.as_deref())
            .and_then(|t| self.unmatched.iter().position(|(id, _)| id == t));
        let (_, d) = match by_id {
            Some(i) => self.unmatched.remove(i)?,
            None => self.unmatched.pop_front()?,
        };
        self.by_agent.insert(normalize_agent_id(agent_id), d.clone());
        Some(d)
    }

    pub fn dispatch_of(&self, agent_id: &str) -> Option<&Dispatch> {
        self.by_agent.get(&normalize_agent_id(agent_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::plan_steps::parse_dispatch;

    /// The fixtures copied once into a tempdir laid out as `<dir>/s.jsonl` and `<dir>/s/subagents/`.
    fn fixture_transcript() -> PathBuf {
        static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
        let dir = DIR.get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let sub = dir.path().join("s").join("subagents");
            std::fs::create_dir_all(&sub).unwrap();
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/hooks/fixtures/subagents");
            for f in ["agent-a1.jsonl", "agent-a1.meta.json"] {
                std::fs::copy(src.join(f), sub.join(f)).unwrap();
            }
            dir
        });
        dir.path().join("s.jsonl")
    }

    #[test]
    fn meta_json_gives_description_and_tool_use_id() {
        let m = read_agent_meta(&fixture_transcript(), "a1").unwrap();
        assert_eq!(
            (
                m.agent_type.as_str(),
                m.description.as_deref(),
                m.tool_use_id.as_deref()
            ),
            (
                "frontend",
                Some("Step 2, 3 · frontend: build the board columns"),
                Some("toolu_01A")
            )
        );
    }

    #[test]
    fn tool_use_ids_in_subagent_transcripts_map_to_their_agent() {
        let idx = ToolUseIndex::scan(&subagent_dir(&fixture_transcript()));
        assert_eq!(idx.agent_for("toolu_01B"), Some("a1"));
        assert_eq!(idx.agent_for("toolu_zzz"), None);
    }

    #[test]
    fn subagent_start_matches_its_dispatch_by_tool_use_id() {
        let mut c = Correlator::default();
        c.note_dispatch("toolu_01A", parse_dispatch("Step 2, 3 · frontend: build").unwrap());
        c.note_dispatch("toolu_01C", parse_dispatch("Step 5 · backend: api").unwrap());
        let meta = AgentMeta {
            agent_id: "a1".into(),
            agent_type: "frontend".into(),
            description: None,
            tool_use_id: Some("toolu_01A".into()),
        };
        assert_eq!(c.on_subagent_start("a1", Some(&meta)).unwrap().steps, vec![2, 3]);
        assert_eq!(c.on_subagent_start("a2", None).unwrap().steps, vec![5]);
    }

    #[test]
    fn without_meta_the_oldest_unmatched_dispatch_is_used() {
        let mut c = Correlator::default();
        c.note_dispatch("x", parse_dispatch("Step 1 · a: one").unwrap());
        c.note_dispatch("y", parse_dispatch("Step 2 · b: two").unwrap());
        assert_eq!(c.on_subagent_start("p", None).unwrap().steps, vec![1]);
        assert_eq!(c.on_subagent_start("q", None).unwrap().steps, vec![2]);
        assert!(c.on_subagent_start("r", None).is_none());
    }

    #[test]
    fn missing_files_never_fail() {
        assert!(read_agent_meta(Path::new("/nowhere/s.jsonl"), "a1").is_none());
        assert_eq!(ToolUseIndex::scan(Path::new("/nowhere")).agent_for("toolu_01B"), None);
    }
}
