//! The pure pieces of "Bring <base> in" (§6.3): reading the session's `Stop` after the integrate prompt, and the
//! other task's intent excerpted from its memory folder for each overlapping path.

use std::collections::BTreeSet;
use std::path::Path;

use chrono::{DateTime, Utc};

use super::prompts::{ConflictCtx, INTEGRATE_SENTINEL};
use super::OverlapRow;
use crate::domain::{AgentState, HandbackReason, Stage, TaskId};
pub use crate::git::valid_branch;
use crate::memory::{IndexEntry, TimelineKind};
use crate::pipeline::Effect;
use crate::settings::Integration;

/// What an integrating session's `Stop` means (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationResult {
    /// The sentinel on a clean tree: back to Idle, recorded in the timeline.
    Done,
    /// Conflict markers or a merge or rebase in progress: `MergeNeedsYou`, whatever the session said.
    Conflicted,
    /// Neither: the session is still at it.
    StillWorking,
}

/// `Conflicted` when the tree has conflicts, regardless of the sentinel; `Done` when the last non-empty line of
/// `message` is the sentinel; otherwise `StillWorking`.
pub fn integration_result(message: &str, has_conflicts: bool) -> IntegrationResult {
    if has_conflicts {
        return IntegrationResult::Conflicted;
    }
    let last = message.lines().map(str::trim).rfind(|l| !l.is_empty());
    if last == Some(INTEGRATE_SENTINEL) {
        IntegrationResult::Done
    } else {
        IntegrationResult::StillWorking
    }
}

/// The other task's intent for `path`: the first `## ` section of its `spec.md`, and from its `plan.md` the
/// `## Files` lines naming `path` followed by the checkbox step lines mentioning it.
pub fn excerpts(path: &str, spec_md: &str, plan_md: &str) -> (String, String) {
    (first_section(spec_md), plan_lines(path, plan_md))
}

/// A `# ` or `## ` heading, which ends a `## ` section (`### ` stays inside it).
fn is_section_heading(line: &str) -> bool {
    line.starts_with("# ") || line.starts_with("## ")
}

fn first_section(md: &str) -> String {
    let mut lines = md.lines().skip_while(|l| !l.starts_with("## "));
    let Some(heading) = lines.next() else {
        return String::new();
    };
    let body: Vec<&str> = lines.take_while(|l| !is_section_heading(l)).collect();
    format!("{heading}\n{}", body.join("\n")).trim().to_owned()
}

fn is_step(line: &str) -> bool {
    let t = line.trim_start();
    ["- [ ]", "- [x]", "- [X]", "* [ ]", "* [x]", "* [X]"]
        .iter()
        .any(|m| t.starts_with(m))
}

fn path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '/')
}

/// `line` names `path` as a whole path: `src/a.ts` is not in `src/a.tsx` or `lib/src/a.ts`, but is in
/// `` `src/a.ts`. `` (a trailing full stop ends the sentence, not the path).
fn mentions(line: &str, path: &str) -> bool {
    !path.is_empty()
        && line.match_indices(path).any(|(i, _)| {
            let before = line[..i].chars().next_back();
            let mut rest = line[i + path.len()..].chars();
            let continues = match rest.next() {
                None => false,
                Some('.') => rest.next().is_some_and(char::is_alphanumeric),
                Some(c) => path_char(c),
            };
            !before.is_some_and(|c| path_char(c) || c == '.') && !continues
        })
}

fn plan_lines(path: &str, md: &str) -> String {
    let (mut files, mut steps) = (Vec::new(), Vec::new());
    let mut in_files = false;
    for line in md.lines() {
        if is_section_heading(line) {
            in_files = line.trim() == "## Files";
            continue;
        }
        if !mentions(line, path) {
            continue;
        }
        if in_files {
            files.push(line.trim_end());
        } else if is_step(line) {
            steps.push(line.trim_end());
        }
    }
    files.extend(steps);
    files.join("\n")
}

/// What `conflict_ctxs` reads of another task: its title and its memory folder's `spec.md` and `plan.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct OtherIntent {
    pub title: String,
    pub spec_md: String,
    pub plan_md: String,
}

/// The integrate prompt's file list (§6.3): one entry per (path, other task) of the task's overlap rows, by path,
/// each with the other task's `excerpts`. A row without another task, or whose task `intent_of` no longer knows,
/// is "another session" with no excerpts.
pub fn conflict_ctxs(rows: &[OverlapRow], intent_of: impl Fn(&TaskId) -> Option<OtherIntent>) -> Vec<ConflictCtx> {
    let mut out = Vec::new();
    let keys: BTreeSet<(&str, Option<&TaskId>)> =
        rows.iter().map(|r| (r.path.as_str(), r.other_task.as_ref())).collect();
    for (path, other) in keys {
        let ctx = match other.and_then(&intent_of) {
            Some(i) => {
                let (spec_excerpt, plan_excerpt) = excerpts(path, &i.spec_md, &i.plan_md);
                ConflictCtx {
                    path: path.to_owned(),
                    other_title: i.title,
                    spec_excerpt,
                    plan_excerpt,
                }
            }
            None => ConflictCtx {
                path: path.to_owned(),
                other_title: "another session".to_owned(),
                spec_excerpt: String::new(),
                plan_excerpt: String::new(),
            },
        };
        out.push(ctx);
    }
    out.sort_by(|a, b| (&a.path, &a.other_title).cmp(&(&b.path, &b.other_title)));
    out
}

/// The effects of an integrating session's `Stop` (§6.3): `Done` returns the task to Idle and records the
/// integration in the timeline (or the abort, when `aborted`); `Conflicted` hands back `MergeNeedsYou` with the
/// conflicted paths of `repo`; `StillWorking` keeps it Working.
pub fn stop_effects(
    result: IntegrationResult,
    base: &str,
    aborted: bool,
    repo: &Path,
    conflicted: Vec<String>,
    stage: Stage,
    now: DateTime<Utc>,
) -> Vec<Effect> {
    match result {
        IntegrationResult::Done => {
            let detail = if aborted {
                format!("aborted bringing {base} in")
            } else {
                format!("integrated {base}")
            };
            vec![
                Effect::SetState(AgentState::Idle),
                Effect::WriteIndex(IndexEntry {
                    at: now,
                    stage,
                    kind: TimelineKind::Overlap { detail },
                }),
            ]
        }
        IntegrationResult::Conflicted => vec![Effect::SetState(AgentState::NeedsYou {
            reason: HandbackReason::MergeNeedsYou {
                repo: repo.to_owned(),
                paths: conflicted,
            },
        })],
        IntegrationResult::StillWorking => vec![Effect::SetState(AgentState::Working)],
    }
}

fn verb(mode: Integration) -> &'static str {
    match mode {
        Integration::Merge => "merge",
        Integration::Rebase => "rebase",
    }
}

fn finish_with_sentinel() -> String {
    format!("end your final message with this single line and nothing after it:\n{INTEGRATE_SENTINEL}\n")
}

/// **Resume** on the merge hand-back (§6.3): the developer's instructions, then the sentinel to end with.
pub fn resume_prompt(instruction: &str, mode: Integration) -> String {
    let verb = verb(mode);
    format!(
        "Fanout Code: continue the integration with these instructions: {}\n\nWhen the {verb} is finished, the \
working tree is clean and the tests pass, {}",
        instruction.trim(),
        finish_with_sentinel()
    )
}

/// **Abort** on the merge hand-back (§6.3): undo the merge or rebase and leave the branch as it was.
pub fn abort_prompt(base: &str, mode: Integration) -> String {
    format!(
        "Fanout Code: the developer asks you to stop bringing `{base}` in. Run `git {} --abort` and leave the branch \
as it was before the {}. Change nothing else. Then {}",
        verb(mode),
        verb(mode),
        finish_with_sentinel()
    )
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::domain::{AgentState, HandbackReason, Stage, TaskId};
    use crate::memory::{IndexEntry, TimelineKind};
    use crate::overlap::{OverlapRow, Relation};
    use crate::pipeline::Effect;

    #[test]
    fn integration_result_rules() {
        let done = "Merged main into fo/login.\nTests pass.\n\nFANOUTCODE: integrate done\n\n";
        assert_eq!(integration_result(done, false), IntegrationResult::Done);
        assert_eq!(integration_result(done, true), IntegrationResult::Conflicted);
        assert_eq!(
            integration_result("I could not keep both in a.ts.", true),
            IntegrationResult::Conflicted
        );
        assert_eq!(
            integration_result("Still merging, running the tests now.", false),
            IntegrationResult::StillWorking
        );
        // the sentinel must be the last line, not merely quoted earlier
        assert_eq!(
            integration_result(
                "I will say FANOUTCODE: integrate done when finished.\nRunning tests.",
                false
            ),
            IntegrationResult::StillWorking
        );
        assert_eq!(integration_result("", false), IntegrationResult::StillWorking);
    }

    #[test]
    fn excerpts_take_spec_section_and_plan_lines() {
        let spec = "# Login\n\nIntro line.\n\n## Summary\n\nUsers sign in with a magic link.\nSessions last a day.\n\n## Details\n\nMore.\n";
        let plan = "# Plan\n\n## Files\n\n- src/auth/session.ts — session expiry\n- src/auth/link.ts\n\n## Steps\n\n- [ ] Add expiry to src/auth/session.ts\n- [x] Send the link from src/auth/link.ts\n- [ ] Test src/auth/session.ts expiry\nNot a step: src/auth/session.ts\n";
        let (s, p) = excerpts("src/auth/session.ts", spec, plan);
        assert_eq!(
            s,
            "## Summary\n\nUsers sign in with a magic link.\nSessions last a day."
        );
        assert_eq!(
            p,
            "- src/auth/session.ts — session expiry\n- [ ] Add expiry to src/auth/session.ts\n- [ ] Test src/auth/session.ts expiry"
        );
        assert_eq!(excerpts("src/x.ts", "no sections", ""), (String::new(), String::new()));
        let near = "## Files\n\n- src/auth/session.tsx\n- lib/src/auth/session.ts\n- `src/auth/session.ts`.\n";
        assert_eq!(excerpts("src/auth/session.ts", "", near).1, "- `src/auth/session.ts`.");
    }

    fn row(path: &str, other: Option<&str>) -> OverlapRow {
        OverlapRow {
            task_id: TaskId::parse("t-a1").unwrap(),
            other_task: other.map(|o| TaskId::parse(o).unwrap()),
            repo_root: "/wt/a".into(),
            path: path.into(),
            relation: Relation::PlannedBy,
            pending: None,
            held: false,
            noted_out: false,
            noted_in: false,
            decided_at: None,
            updated_at: "2026-10-25T10:00:00Z".parse().unwrap(),
        }
    }

    #[test]
    fn conflict_ctx_round_trips_through_serde() {
        let c = ConflictCtx {
            path: "src/a.ts".into(),
            other_title: "Login".into(),
            spec_excerpt: "## Summary\nSign in.".into(),
            plan_excerpt: "- src/a.ts".into(),
        };
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["other_title"], "Login");
        assert_eq!(serde_json::from_value::<ConflictCtx>(json).unwrap(), c);
    }

    #[test]
    fn conflict_ctxs_one_per_path_and_task_with_both_excerpts() {
        let rows = [
            row("src/b.ts", Some("t-b2")),
            row("src/a.ts", Some("t-b2")),
            row("src/a.ts", Some("t-b2")),
            row("src/a.ts", None),
            row("src/c.ts", Some("t-c3")),
        ];
        let intent_of = |id: &TaskId| {
            (id.as_str() == "t-b2").then(|| OtherIntent {
                title: "Login".into(),
                spec_md: "# Login\n\n## Summary\nSign in.\n".into(),
                plan_md: "## Files\n- src/a.ts — form\n- src/b.ts\n".into(),
            })
        };
        let got = conflict_ctxs(&rows, intent_of);
        let keys: Vec<(&str, &str)> = got.iter().map(|c| (c.path.as_str(), c.other_title.as_str())).collect();
        assert_eq!(
            keys,
            [
                ("src/a.ts", "Login"),
                ("src/a.ts", "another session"),
                ("src/b.ts", "Login"),
                ("src/c.ts", "another session"),
            ]
        );
        assert_eq!(got[0].spec_excerpt, "## Summary\nSign in.");
        assert_eq!(got[0].plan_excerpt, "- src/a.ts — form");
        assert_eq!((got[1].spec_excerpt.as_str(), got[1].plan_excerpt.as_str()), ("", ""));
        assert!(conflict_ctxs(&[], intent_of).is_empty());
    }

    #[test]
    fn stop_effects_rules() {
        let now: DateTime<Utc> = "2026-10-25T10:00:00Z".parse().unwrap();
        let repo = Path::new("/wt/a");
        let entry = |detail: &str| {
            Effect::WriteIndex(IndexEntry {
                at: now,
                stage: Stage::Done,
                kind: TimelineKind::Overlap { detail: detail.into() },
            })
        };
        assert_eq!(
            stop_effects(IntegrationResult::Done, "main", false, repo, vec![], Stage::Done, now),
            [Effect::SetState(AgentState::Idle), entry("integrated main")]
        );
        assert_eq!(
            stop_effects(IntegrationResult::Done, "main", true, repo, vec![], Stage::Done, now),
            [Effect::SetState(AgentState::Idle), entry("aborted bringing main in")]
        );
        assert_eq!(
            stop_effects(
                IntegrationResult::Conflicted,
                "main",
                false,
                repo,
                vec!["src/a.ts".into()],
                Stage::Done,
                now
            ),
            [Effect::SetState(AgentState::NeedsYou {
                reason: HandbackReason::MergeNeedsYou {
                    repo: repo.into(),
                    paths: vec!["src/a.ts".into()],
                },
            })]
        );
        assert_eq!(
            stop_effects(
                IntegrationResult::StillWorking,
                "main",
                false,
                repo,
                vec![],
                Stage::Done,
                now
            ),
            [Effect::SetState(AgentState::Working)]
        );
    }

    #[test]
    fn resume_and_abort_prompts_end_with_the_sentinel() {
        let resume = resume_prompt("  keep the new session expiry from Login  ", Integration::Merge);
        assert!(resume.starts_with(
            "Fanout Code: continue the integration with these instructions: keep the new session expiry from Login\n"
        ));
        assert!(resume.contains("When the merge is finished"), "{resume}");
        let abort = abort_prompt("main", Integration::Rebase);
        assert!(
            abort.contains("`git rebase --abort`") && abort.contains("`main`"),
            "{abort}"
        );
        let merge_abort = abort_prompt("main", Integration::Merge);
        assert!(merge_abort.contains("`git merge --abort`"), "{merge_abort}");
        for p in [resume, abort, merge_abort] {
            assert_eq!(p.trim_end().lines().last(), Some(INTEGRATE_SENTINEL));
            assert!(!p.contains("commit") && !p.contains("push"), "{p}");
        }
    }
}
