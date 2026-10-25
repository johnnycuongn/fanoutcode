//! The Build conventions (§6.3, contracts): `plan.md` steps `- [ ] 5. Title (after 2, 7)`, the `## Team` section,
//! and subagent dispatch descriptions `Step 5 · frontend: …` / `Step 5, 6 · frontend: …`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One numbered checkbox step of `plan.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PlanStep {
    pub number: u32,
    pub title: String,
    pub done: bool,
    /// The `(after …)` list, else the previous step, else nothing.
    pub deps: Vec<u32>,
    /// `deps` came from an `(after …)` list.
    pub explicit_deps: bool,
    /// The linked project heading the step sits under; `None` for the home project.
    pub repo: Option<String>,
}

/// One bullet of the `## Team` section: a role and the steps it owns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct TeamRole {
    pub role: String,
    pub steps: Vec<u32>,
}

/// `plan.md` as the Team canvas reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct PlanDoc {
    pub steps: Vec<PlanStep>,
    pub team: Vec<TeamRole>,
}

/// A subagent dispatch: the steps it covers, its role and the rest of its description.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub struct Dispatch {
    pub steps: Vec<u32>,
    pub role: String,
    pub summary: String,
    pub subagent_type: Option<String>,
}

/// `##` headings that are sections of the plan, not linked projects.
const RESERVED: [&str; 4] = ["steps", "team", "files", "tests"];

/// `2, 3` or `2 and 3` as numbers; `None` when any part is not a number or there are none.
fn numbers(s: &str) -> Option<Vec<u32>> {
    let out: Option<Vec<u32>> = s
        .split([',', ' '])
        .map(str::trim)
        .filter(|p| !p.is_empty() && *p != "and" && *p != "&")
        .map(|p| p.parse().ok())
        .collect();
    out.filter(|v| !v.is_empty())
}

/// `- [ ] N. Title (after a, b)` → `(number, title, done, explicit deps)`.
fn step_line(line: &str) -> Option<(u32, String, bool, Option<Vec<u32>>)> {
    let rest = line.trim_start().strip_prefix("- [")?;
    let (mark, rest) = rest.split_once(']')?;
    let done = match mark {
        " " => false,
        "x" | "X" => true,
        _ => return None,
    };
    let (num, title) = rest.trim_start().split_once('.')?;
    let number: u32 = num.trim().parse().ok()?;
    let mut title = title.trim().to_owned();
    let mut deps = None;
    if let Some(open) = title.rfind("(after ") {
        if title.ends_with(')') {
            deps = numbers(&title[open + "(after ".len()..title.len() - 1]);
            if deps.is_some() {
                title = title[..open].trim_end().to_owned();
            }
        }
    }
    Some((number, title, done, deps))
}

/// `- frontend: steps 2, 3, 4` → the role and its steps.
fn team_line(line: &str) -> Option<TeamRole> {
    let rest = line.trim_start().strip_prefix("- ")?;
    let (role, steps) = rest.split_once(':')?;
    let steps = steps.trim();
    let steps = steps
        .strip_prefix("steps")
        .or_else(|| steps.strip_prefix("step"))
        .or_else(|| steps.strip_prefix("Steps"))
        .or_else(|| steps.strip_prefix("Step"))
        .unwrap_or(steps);
    Some(TeamRole {
        role: role.trim().to_lowercase(),
        steps: numbers(steps)?,
    })
}

/// Reads the steps, their dependencies and repos, and the Team section. Lines that fit neither grammar are ignored.
pub fn parse_plan(md: &str) -> PlanDoc {
    let mut doc = PlanDoc::default();
    let (mut repo, mut in_team) = (None::<String>, false);
    for line in md.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let heading = heading.trim();
            let reserved = RESERVED.contains(&heading.to_lowercase().as_str());
            in_team = heading.eq_ignore_ascii_case("team");
            repo = (!reserved).then(|| heading.to_owned());
            continue;
        }
        if in_team {
            doc.team.extend(team_line(line));
        } else if let Some((number, title, done, explicit)) = step_line(line) {
            let explicit_deps = explicit.is_some();
            let deps = explicit.unwrap_or_else(|| doc.steps.last().map(|s| vec![s.number]).unwrap_or_default());
            doc.steps.push(PlanStep {
                number,
                title,
                done,
                deps,
                explicit_deps,
                repo: repo.clone(),
            });
        }
    }
    doc
}

/// `Step N[, M…] <· | - | —> role: summary`; `None` for any other description.
pub fn parse_dispatch(text: &str) -> Option<Dispatch> {
    let t = text.trim();
    let rest = t.strip_prefix("Steps ").or_else(|| t.strip_prefix("Step "))?;
    let sep = rest.find(['·', '-', '—'])?;
    let steps = numbers(&rest[..sep])?;
    let after = &rest[sep..];
    let after = after[after.chars().next()?.len_utf8()..].trim_start();
    let (role, summary) = after.split_once(':')?;
    let role = role.trim().to_lowercase();
    if role.is_empty() {
        return None;
    }
    Some(Dispatch {
        steps,
        role,
        summary: summary.trim().to_owned(),
        subagent_type: None,
    })
}

/// A `Task` or `Agent` tool call as a dispatch: its description parsed, else the subagent type as the role (or
/// `general-purpose`) with the whole description as the summary. `None` for any other tool.
pub fn dispatch_from_tool_input(tool: &str, input: &Value) -> Option<Dispatch> {
    if tool != "Task" && tool != "Agent" {
        return None;
    }
    let description = input.get("description").and_then(Value::as_str).unwrap_or("");
    let subagent_type = input.get("subagent_type").and_then(Value::as_str).map(str::to_owned);
    Some(match parse_dispatch(description) {
        Some(d) => Dispatch { subagent_type, ..d },
        None => Dispatch {
            steps: Vec::new(),
            role: subagent_type.clone().unwrap_or_else(|| "general-purpose".to_owned()),
            summary: description.to_owned(),
            subagent_type,
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const PLAN: &str = include_str!("fixtures/plan.md");

    #[test]
    fn checkbox_steps_parse_number_title_and_done() {
        let d = parse_plan(PLAN);
        assert_eq!(d.steps.len(), 5);
        assert_eq!(
            (d.steps[0].number, d.steps[0].title.as_str(), d.steps[0].done),
            (1, "Scaffold the board feature", true)
        );
        assert_eq!(
            (d.steps[1].number, d.steps[1].title.as_str(), d.steps[1].done),
            (2, "Build the board columns", false)
        );
    }

    #[test]
    fn after_lists_become_explicit_deps() {
        let d = parse_plan(PLAN);
        assert_eq!(d.steps[3].deps, vec![2, 3]);
        assert!(d.steps[3].explicit_deps);
    }

    #[test]
    fn a_step_without_after_follows_the_previous_step() {
        let d = parse_plan(PLAN);
        assert_eq!(d.steps[2].deps, vec![2]);
        assert!(!d.steps[2].explicit_deps);
        assert!(d.steps[0].deps.is_empty());
    }

    #[test]
    fn steps_under_a_project_heading_carry_the_repo() {
        let d = parse_plan(PLAN);
        assert_eq!(d.steps[4].repo.as_deref(), Some("api-server"));
        assert_eq!(d.steps[1].repo, None);
    }

    #[test]
    fn team_section_lists_roles_and_steps() {
        let d = parse_plan(PLAN);
        assert_eq!(
            d.team,
            vec![
                TeamRole {
                    role: "frontend".into(),
                    steps: vec![2, 3, 4]
                },
                TeamRole {
                    role: "backend".into(),
                    steps: vec![5]
                }
            ]
        );
    }

    #[test]
    fn dispatch_parses_one_or_several_steps() {
        assert_eq!(
            parse_dispatch("Step 5 · frontend: build it").unwrap(),
            Dispatch {
                steps: vec![5],
                role: "frontend".into(),
                summary: "build it".into(),
                subagent_type: None
            }
        );
        assert_eq!(
            parse_dispatch("Step 5, 6 · frontend: build it").unwrap().steps,
            vec![5, 6]
        );
        assert_eq!(parse_dispatch("hello").unwrap_or_default().steps, Vec::<u32>::new());
    }

    #[test]
    fn dispatch_without_steps_falls_back_to_subagent_type() {
        let d = dispatch_from_tool_input(
            "Agent",
            &json!({"description": "Review the diff", "subagent_type": "security reviewer"}),
        )
        .unwrap();
        assert_eq!(
            (d.steps.len(), d.role.as_str(), d.summary.as_str()),
            (0, "security reviewer", "Review the diff")
        );
    }

    #[test]
    fn only_task_and_agent_tools_are_dispatches() {
        assert!(dispatch_from_tool_input("Task", &json!({"description": "Step 1 · backend: x"})).is_some());
        assert!(dispatch_from_tool_input("Bash", &json!({"command": "ls"})).is_none());
    }
}
