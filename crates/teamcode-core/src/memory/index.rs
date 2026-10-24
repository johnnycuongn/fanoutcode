//! `index.md`: the front matter and `## Timeline` of a task folder (contracts "Memory folder"). Only the app writes
//! it. A hand-rolled line format, no YAML crate: the shape is fixed and small.

use std::path::PathBuf;

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

use crate::domain::{AgentKind, AgentState, ProjectId, RepoBase, Stage, TaskId, WorkspaceId};

/// The front matter keys, in file order. `render` and `parse` share it.
const KEYS: [&str; 14] = [
    "id",
    "project",
    "workspace",
    "title",
    "stage",
    "state",
    "jira",
    "agent",
    "branch",
    "worktree",
    "base",
    "claude_sessions",
    "created",
    "updated",
];

/// Timeline lines use minutes: `2026-10-24T05:12Z`.
const MINUTE_FORMAT: &str = "%Y-%m-%dT%H:%MZ";

#[derive(Debug, Clone, PartialEq)]
pub struct IndexFrontMatter {
    pub id: TaskId,
    pub project: ProjectId,
    pub workspace: Option<WorkspaceId>,
    pub title: String,
    pub stage: Stage,
    pub state: StateWord,
    pub jira: Option<String>,
    pub agent: AgentKind,
    pub branch: Option<String>,
    pub worktree: Option<PathBuf>,
    pub base: Vec<RepoBase>,
    pub claude_sessions: Vec<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

/// The agent state as one word: `working` | `needs-you` | `idle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateWord {
    Working,
    NeedsYou,
    Idle,
}

impl StateWord {
    pub fn from_state(state: &AgentState) -> StateWord {
        match state {
            AgentState::Working => StateWord::Working,
            AgentState::NeedsYou { .. } => StateWord::NeedsYou,
            AgentState::Idle => StateWord::Idle,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            StateWord::Working => "working",
            StateWord::NeedsYou => "needs-you",
            StateWord::Idle => "idle",
        }
    }

    pub fn parse(s: &str) -> Option<StateWord> {
        Some(match s {
            "working" => StateWord::Working,
            "needs-you" => StateWord::NeedsYou,
            "idle" => StateWord::Idle,
            _ => return None,
        })
    }
}

/// One timeline line (the contract's `Effect::WriteIndex(IndexEntry)`).
#[derive(Debug, Clone, PartialEq)]
pub struct IndexEntry {
    pub at: DateTime<Utc>,
    pub stage: Stage,
    pub kind: TimelineKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineKind {
    Created,
    Entered,
    Exited,
    Skipped,
    Handback {
        chip: String,
    },
    Commit {
        repo: String,
        sha: String,
    },
    /// The spec watcher rewrote `spec.md` (Plan 05): `· spec: <reason>`.
    Spec {
        reason: String,
    },
    Archived,
    Restored,
}

impl TimelineKind {
    /// The word stored in `task_timeline.kind` and leading each timeline line.
    pub fn word(&self) -> &'static str {
        match self {
            TimelineKind::Created => "created",
            TimelineKind::Entered => "entered",
            TimelineKind::Exited => "exited",
            TimelineKind::Skipped => "skipped",
            TimelineKind::Handback { .. } => "handback",
            TimelineKind::Commit { .. } => "commit",
            TimelineKind::Spec { .. } => "spec",
            TimelineKind::Archived => "archived",
            TimelineKind::Restored => "restored",
        }
    }

    /// The unit kinds from their word; `Handback` and `Commit` need their details.
    pub fn unit_from_word(word: &str) -> Option<TimelineKind> {
        Some(match word {
            "created" => TimelineKind::Created,
            "entered" => TimelineKind::Entered,
            "exited" => TimelineKind::Exited,
            "skipped" => TimelineKind::Skipped,
            "archived" => TimelineKind::Archived,
            "restored" => TimelineKind::Restored,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDoc {
    pub front: IndexFrontMatter,
    pub timeline: Vec<IndexEntry>,
}

fn agent_word(kind: AgentKind) -> &'static str {
    match kind {
        AgentKind::ClaudeCode => "claude-code",
        AgentKind::Codex => "codex",
        AgentKind::Shell => "shell",
    }
}

fn parse_agent(s: &str) -> Option<AgentKind> {
    Some(match s {
        "claude-code" => AgentKind::ClaudeCode,
        "codex" => AgentKind::Codex,
        "shell" => AgentKind::Shell,
        _ => return None,
    })
}

fn seconds(t: &DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// `key: value`, or the bare `key:` when the value is empty.
fn line(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push(':');
    if !value.is_empty() {
        out.push(' ');
        out.push_str(value);
    }
    out.push('\n');
}

/// The whole file: front matter in `KEYS` order, then `## Timeline`.
pub fn render(doc: &IndexDoc) -> String {
    let f = &doc.front;
    let mut out = String::from("---\n");
    for key in KEYS {
        match key {
            "id" => line(&mut out, key, f.id.as_str()),
            "project" => line(&mut out, key, f.project.as_str()),
            "workspace" => line(&mut out, key, f.workspace.as_ref().map_or("", |w| w.as_str())),
            // a title is one line; a stray newline would end the front matter
            "title" => line(&mut out, key, &f.title.replace(['\n', '\r'], " ")),
            "stage" => line(&mut out, key, f.stage.as_str()),
            "state" => line(&mut out, key, f.state.as_str()),
            "jira" => line(&mut out, key, f.jira.as_deref().unwrap_or("")),
            "agent" => line(&mut out, key, agent_word(f.agent)),
            "branch" => line(&mut out, key, f.branch.as_deref().unwrap_or("")),
            "worktree" => {
                let path = f.worktree.as_ref().map(|p| p.display().to_string());
                line(&mut out, key, path.as_deref().unwrap_or("off"))
            }
            "base" => {
                line(&mut out, key, "");
                for b in &f.base {
                    out.push_str(&format!("  {}: {}\n", b.repo_root.display(), b.commit));
                }
            }
            "claude_sessions" => {
                line(&mut out, key, "");
                for s in &f.claude_sessions {
                    out.push_str(&format!("  - {s}\n"));
                }
            }
            "created" => line(&mut out, key, &seconds(&f.created)),
            "updated" => line(&mut out, key, &seconds(&f.updated)),
            _ => unreachable!("every key in KEYS is rendered"),
        }
    }
    out.push_str("---\n\n## Timeline\n\n");
    for e in &doc.timeline {
        out.push_str(&render_entry(e));
        out.push('\n');
    }
    out
}

/// One timeline line, no trailing newline: `- 2026-10-24T05:12Z · plan · entered`.
pub fn render_entry(e: &IndexEntry) -> String {
    let kind = match &e.kind {
        TimelineKind::Handback { chip } => format!("handback: {chip}"),
        TimelineKind::Commit { repo, sha } => format!("commit: {repo} {sha}"),
        TimelineKind::Spec { reason } => format!("spec: {reason}"),
        other => other.word().to_owned(),
    };
    format!("- {} · {} · {kind}", e.at.format(MINUTE_FORMAT), e.stage.as_str())
}

fn parse_entry(line: &str) -> Option<IndexEntry> {
    let mut parts = line.strip_prefix("- ")?.splitn(3, " · ");
    let at = NaiveDateTime::parse_from_str(parts.next()?, MINUTE_FORMAT)
        .ok()?
        .and_utc();
    let stage = Stage::parse(parts.next()?)?;
    let kind = parts.next()?;
    let kind = if let Some(chip) = kind.strip_prefix("handback: ") {
        TimelineKind::Handback { chip: chip.to_owned() }
    } else if let Some(reason) = kind.strip_prefix("spec: ") {
        TimelineKind::Spec {
            reason: reason.to_owned(),
        }
    } else if let Some(rest) = kind.strip_prefix("commit: ") {
        let (repo, sha) = rest.rsplit_once(' ')?;
        TimelineKind::Commit {
            repo: repo.to_owned(),
            sha: sha.to_owned(),
        }
    } else {
        TimelineKind::unit_from_word(kind)?
    };
    Some(IndexEntry { at, stage, kind })
}

/// A front matter key, its inline value (empty for a bare `key:`) and its indented child lines.
struct Field<'a> {
    key: &'a str,
    value: &'a str,
    children: Vec<&'a str>,
}

fn parse_fields(front: &str) -> Option<Vec<Field<'_>>> {
    let mut fields: Vec<Field> = Vec::new();
    for line in front.lines() {
        if let Some(child) = line.strip_prefix("  ") {
            fields.last_mut()?.children.push(child);
        } else if let Some(key) = line.strip_suffix(':').filter(|k| !k.contains(": ")) {
            fields.push(Field {
                key,
                value: "",
                children: Vec::new(),
            });
        } else {
            let (key, value) = line.split_once(": ")?;
            fields.push(Field {
                key,
                value,
                children: Vec::new(),
            });
        }
    }
    Some(fields)
}

fn time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|t| t.with_timezone(&Utc))
}

fn optional(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_owned())
}

/// `None` when the front matter is missing or unreadable; a timeline line that does not parse is skipped.
pub fn parse(text: &str) -> Option<IndexDoc> {
    let rest = text.strip_prefix("---\n")?;
    let (front, body) = rest.split_once("\n---\n")?;
    let fields = parse_fields(front)?;
    let get = |key: &str| fields.iter().find(|f| f.key == key);
    let value = |key: &str| get(key).map(|f| f.value);
    let workspace = value("workspace")?;
    let worktree = value("worktree")?;
    let front = IndexFrontMatter {
        id: TaskId::parse(value("id")?).ok()?,
        project: ProjectId::parse(value("project")?).ok()?,
        workspace: if workspace.is_empty() {
            None
        } else {
            Some(WorkspaceId::parse(workspace).ok()?)
        },
        title: value("title")?.to_owned(),
        stage: Stage::parse(value("stage")?)?,
        state: StateWord::parse(value("state")?)?,
        jira: optional(value("jira")?),
        agent: parse_agent(value("agent")?)?,
        branch: optional(value("branch")?),
        worktree: (!worktree.is_empty() && worktree != "off").then(|| PathBuf::from(worktree)),
        base: get("base")?
            .children
            .iter()
            .map(|c| {
                let (root, sha) = c.rsplit_once(": ")?;
                Some(RepoBase {
                    repo_root: root.into(),
                    commit: sha.to_owned(),
                })
            })
            .collect::<Option<_>>()?,
        claude_sessions: get("claude_sessions")?
            .children
            .iter()
            .map(|c| c.strip_prefix("- ").map(str::to_owned))
            .collect::<Option<_>>()?,
        created: time(value("created")?)?,
        updated: time(value("updated")?)?,
    };
    let timeline = body
        .lines()
        .filter(|l| l.starts_with("- "))
        .filter_map(parse_entry)
        .collect();
    Some(IndexDoc { front, timeline })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> DateTime<Utc> {
        "2026-10-24T05:12:33Z".parse().unwrap()
    }

    fn sample() -> IndexDoc {
        let entry = |stage, kind| IndexEntry { at: at(), stage, kind };
        IndexDoc {
            front: IndexFrontMatter {
                id: TaskId::parse("t-1a-1").unwrap(),
                project: ProjectId::parse("p-2b-1").unwrap(),
                workspace: Some(WorkspaceId::parse("ws-3c-1").unwrap()),
                title: "Add dark: mode".into(),
                stage: Stage::Plan,
                state: StateWord::NeedsYou,
                jira: Some("TC-12".into()),
                agent: AgentKind::ClaudeCode,
                branch: Some("tc/add-dark-mode".into()),
                worktree: Some("/r/.worktrees/add-dark-mode".into()),
                base: vec![RepoBase {
                    repo_root: "/r".into(),
                    commit: "abc123".into(),
                }],
                claude_sessions: vec!["s-1".into()],
                created: at(),
                updated: at(),
            },
            timeline: vec![
                entry(Stage::Spec, TimelineKind::Created),
                entry(Stage::Spec, TimelineKind::Entered),
                entry(Stage::Spec, TimelineKind::Exited),
                entry(Stage::Plan, TimelineKind::Entered),
                entry(
                    Stage::Plan,
                    TimelineKind::Handback {
                        chip: "Plan written, go?".into(),
                    },
                ),
            ],
        }
    }

    #[test]
    fn render_matches_the_contract_shape() {
        let text = render(&sample());
        assert!(text.starts_with("---\nid: t-1a-1\nproject: p-2b-1\nworkspace: ws-3c-1\ntitle: Add dark: mode\nstage: plan\nstate: needs-you\njira: TC-12\nagent: claude-code\nbranch: tc/add-dark-mode\nworktree: /r/.worktrees/add-dark-mode\nbase:\n  /r: abc123\nclaude_sessions:\n  - s-1\ncreated: 2026-10-24T05:12:33Z\nupdated: 2026-10-24T05:12:33Z\n---\n\n## Timeline\n\n"), "{text}");
        assert!(text.contains("\n- 2026-10-24T05:12Z · plan · handback: Plan written, go?\n"));
        assert!(text.ends_with("\n"));
    }

    #[test]
    fn render_then_parse_round_trips() {
        let doc = sample();
        let back = parse(&render(&doc)).unwrap();
        assert_eq!(back.front.title, "Add dark: mode");
        assert_eq!(back.front.workspace.as_ref().map(|w| w.as_str()), Some("ws-3c-1"));
        assert_eq!(back.front.base, doc.front.base);
        assert_eq!(back.timeline.len(), 5);
        assert_eq!(
            back.timeline[4].kind,
            TimelineKind::Handback {
                chip: "Plan written, go?".into()
            }
        );
        assert_eq!(back.timeline[4].at.to_rfc3339(), "2026-10-24T05:12:00+00:00");
    }

    #[test]
    fn empty_optionals_render_bare_keys_and_parse_back_as_none() {
        let mut doc = sample();
        doc.front.workspace = None;
        doc.front.jira = None;
        doc.front.branch = None;
        doc.front.worktree = None;
        doc.front.base.clear();
        doc.front.claude_sessions.clear();
        let text = render(&doc);
        assert!(
            text.contains("\nworkspace:\n")
                && text.contains("\nworktree: off\n")
                && text.contains("\nbase:\nclaude_sessions:\n")
        );
        let back = parse(&text).unwrap();
        assert!(back.front.workspace.is_none() && back.front.worktree.is_none() && back.front.base.is_empty());
    }

    #[test]
    fn parse_rejects_a_file_without_front_matter_and_skips_bad_timeline_lines() {
        assert!(parse("## Timeline\n- junk\n").is_none());
        let text = render(&sample()).replace("- 2026-10-24T05:12Z · spec · exited", "- not a line");
        assert_eq!(parse(&text).unwrap().timeline.len(), 4);
    }

    #[test]
    fn render_entry_formats_commits() {
        let e = IndexEntry {
            at: "2026-10-24T05:12:33Z".parse().unwrap(),
            stage: Stage::Commit,
            kind: TimelineKind::Commit {
                repo: "/r".into(),
                sha: "abc123".into(),
            },
        };
        assert_eq!(render_entry(&e), "- 2026-10-24T05:12Z · commit · commit: /r abc123");
    }

    #[test]
    fn a_spec_watcher_line_renders_and_parses_back() {
        let e = IndexEntry {
            at: "2026-10-24T05:12:33Z".parse().unwrap(),
            stage: Stage::Review,
            kind: TimelineKind::Spec {
                reason: "rows are now collapsible".into(),
            },
        };
        let line = render_entry(&e);
        assert_eq!(line, "- 2026-10-24T05:12Z · review · spec: rows are now collapsible");
        assert_eq!(parse_entry(&line).unwrap().kind, e.kind);
    }
}
