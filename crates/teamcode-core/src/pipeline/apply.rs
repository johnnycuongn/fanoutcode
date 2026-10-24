//! Applying a transition's effects: the persistence-shaped ones go to SQLite in one transaction and are mirrored to
//! `index.md`; the external ones go back to the shell, order kept, for Plans 04 and 05 to run.

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use super::handback::chip_text;
use super::Effect;
use crate::db;
use crate::domain::{AgentState, Task};
use crate::memory::{IndexDoc, IndexEntry, IndexFrontMatter, StateWord, TaskFolder, TimelineKind};
use crate::Result;

/// The task after the effects, and the effects the shell must run, in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub task: Task,
    pub external: Vec<Effect>,
}

/// Persists the stage, state, archive flag and timeline in one SQLite transaction, then rewrites `index.md` from
/// the saved row. The database is the source and the file its mirror: a file write failure is returned only after
/// the row is saved. A missing folder is skipped, never recreated (spec §8); a corrupt `index.md` is set aside as
/// `index.md.bad` before the rewrite.
pub fn apply(
    conn: &mut Connection,
    folder: &TaskFolder,
    task: &Task,
    effects: Vec<Effect>,
    now: DateTime<Utc>,
) -> Result<Applied> {
    let mut t = task.clone();
    let mut external = Vec::new();
    let tx = conn.transaction()?;
    let entry = |stage, kind| IndexEntry { at: now, stage, kind };
    for effect in effects {
        match effect {
            Effect::SetStage(stage) => {
                db::timeline::append(&tx, &t.id, &entry(t.stage, TimelineKind::Exited))?;
                db::timeline::append(&tx, &t.id, &entry(stage, TimelineKind::Entered))?;
                t.stage = stage;
                t.stage_entered_at = now;
            }
            Effect::SetState(state) if state != t.state => {
                if let AgentState::NeedsYou { reason } = &state {
                    let chip = chip_text(reason, t.stage);
                    db::timeline::append(&tx, &t.id, &entry(t.stage, TimelineKind::Handback { chip }))?;
                }
                t.state = state;
                t.state_entered_at = now;
            }
            Effect::SetState(_) => {}
            Effect::SetArchived(archived) => t.archived = archived,
            Effect::WriteIndex(e) => db::timeline::append(&tx, &t.id, &e)?,
            other => external.push(other),
        }
    }
    t.updated_at = now;
    db::tasks::update(&tx, &t)?;
    tx.commit()?;
    if !folder.is_missing() {
        // reading sets a corrupt file aside, so the rewrite never destroys it
        folder.read_index()?;
        folder.write_index(&index_doc_for(&t, db::timeline::list(conn, &t.id)?))?;
    }
    Ok(Applied { task: t, external })
}

/// The `index.md` document for a task row and its timeline.
pub fn index_doc_for(task: &Task, timeline: Vec<IndexEntry>) -> IndexDoc {
    IndexDoc {
        front: IndexFrontMatter {
            id: task.id.clone(),
            project: task.project_id.clone(),
            workspace: task.workspace_id.clone(),
            title: task.title.clone(),
            stage: task.stage,
            state: StateWord::from_state(&task.state),
            jira: task.jira_key.clone(),
            agent: task.agent_kind,
            branch: task.branch.clone(),
            worktree: task.worktree_path.clone(),
            base: task.base.clone(),
            claude_sessions: task.claude_session_id.iter().cloned().collect(),
            created: task.created_at,
            updated: task.updated_at,
        },
        timeline,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::domain::{HandbackReason, Stage};
    use crate::memory::MemoryFile;
    use crate::test_support::setup;

    #[test]
    fn set_stage_records_exit_and_entry_and_moves_the_task() {
        let (mut c, _dir, folder, t) = setup(Stage::Spec);
        let now: DateTime<Utc> = "2026-10-24T06:00:00Z".parse().unwrap();
        let a = apply(
            &mut c,
            &folder,
            &t,
            vec![
                Effect::SetStage(Stage::Plan),
                Effect::StartSession,
                Effect::InjectPrompt("P".into()),
                Effect::SetState(AgentState::Working),
            ],
            now,
        )
        .unwrap();
        assert_eq!(
            (a.task.stage, a.task.stage_entered_at, a.task.state.clone()),
            (Stage::Plan, now, AgentState::Working)
        );
        assert_eq!(a.external, [Effect::StartSession, Effect::InjectPrompt("P".into())]);
        let kinds: Vec<(Stage, TimelineKind)> = db::timeline::list(&c, &t.id)
            .unwrap()
            .into_iter()
            .map(|e| (e.stage, e.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                (Stage::Spec, TimelineKind::Exited),
                (Stage::Plan, TimelineKind::Entered)
            ]
        );
        assert_eq!(db::tasks::get(&c, &t.id).unwrap().unwrap().stage, Stage::Plan);
        let text = folder.read(MemoryFile::Index).unwrap().unwrap();
        assert!(text.contains("stage: plan\nstate: working\n") && text.contains("· plan · entered"));
    }

    #[test]
    fn needs_you_state_writes_a_handback_line_with_the_chip_text() {
        let (mut c, _dir, folder, t) = setup(Stage::Plan);
        apply(
            &mut c,
            &folder,
            &t,
            vec![Effect::SetState(AgentState::NeedsYou {
                reason: HandbackReason::StageDone,
            })],
            Utc::now(),
        )
        .unwrap();
        let last = db::timeline::list(&c, &t.id).unwrap().pop().unwrap();
        assert_eq!(
            last.kind,
            TimelineKind::Handback {
                chip: "Plan written, go?".into()
            }
        );
        assert!(folder
            .read(MemoryFile::Index)
            .unwrap()
            .unwrap()
            .contains("· plan · handback: Plan written, go?"));
    }

    #[test]
    fn state_entered_at_moves_on_a_state_change_only() {
        let (mut c, _dir, folder, t) = setup(Stage::Build);
        let stage_at = t.stage_entered_at;
        let now: DateTime<Utc> = "2026-10-24T07:00:00Z".parse().unwrap();
        let a = apply(&mut c, &folder, &t, vec![Effect::SetState(AgentState::Working)], now).unwrap();
        assert_eq!((a.task.state_entered_at, a.task.stage_entered_at), (now, stage_at));
        let later: DateTime<Utc> = "2026-10-24T07:05:00Z".parse().unwrap();
        let b = apply(
            &mut c,
            &folder,
            &a.task,
            vec![Effect::SetState(AgentState::Working)],
            later,
        )
        .unwrap();
        assert_eq!(b.task.state_entered_at, now, "same state again is not a change");
        let r = AgentState::NeedsYou {
            reason: HandbackReason::Replied {
                first_line: "Which DB?".into(),
            },
        };
        let d = apply(&mut c, &folder, &b.task, vec![Effect::SetState(r)], later).unwrap();
        assert_eq!(db::tasks::get(&c, &t.id).unwrap().unwrap().state_entered_at, later);
        assert_eq!(d.task.stage_entered_at, stage_at);
    }

    #[test]
    fn archive_flag_and_explicit_index_entries_persist() {
        let (mut c, _dir, folder, t) = setup(Stage::Build);
        let e = IndexEntry {
            at: Utc::now(),
            stage: Stage::Build,
            kind: TimelineKind::Archived,
        };
        let a = apply(
            &mut c,
            &folder,
            &t,
            vec![Effect::EndSession, Effect::SetArchived(true), Effect::WriteIndex(e)],
            Utc::now(),
        )
        .unwrap();
        assert!(a.task.archived && a.external == [Effect::EndSession]);
        assert_eq!(
            db::timeline::list(&c, &t.id).unwrap().last().unwrap().kind,
            TimelineKind::Archived
        );
    }

    #[test]
    fn missing_folder_still_persists_to_sqlite_and_does_not_recreate_it() {
        let (mut c, _dir, folder, t) = setup(Stage::Spec);
        std::fs::remove_dir_all(folder.dir()).unwrap();
        let a = apply(&mut c, &folder, &t, vec![Effect::SetStage(Stage::Plan)], Utc::now()).unwrap();
        assert_eq!(a.task.stage, Stage::Plan);
        assert!(!folder.dir().exists());
        assert_eq!(db::tasks::get(&c, &t.id).unwrap().unwrap().stage, Stage::Plan);
    }

    #[test]
    fn a_corrupt_index_is_set_aside_before_the_rewrite() {
        let (mut c, _dir, folder, t) = setup(Stage::Spec);
        std::fs::write(folder.path(MemoryFile::Index), "not front matter").unwrap();
        apply(&mut c, &folder, &t, vec![Effect::SetStage(Stage::Plan)], Utc::now()).unwrap();
        assert_eq!(
            std::fs::read_to_string(folder.dir().join("index.md.bad")).unwrap(),
            "not front matter"
        );
        assert!(folder.read(MemoryFile::Index).unwrap().unwrap().contains("stage: plan"));
    }

    #[test]
    fn index_doc_mirrors_the_task_row() {
        let (_c, _dir, _f, mut t) = setup(Stage::Review);
        t.claude_session_id = Some("abc".into());
        t.worktree_path = Some("/r/.worktrees/x".into());
        t.branch = Some("tc/x".into());
        let doc = index_doc_for(&t, vec![]);
        assert_eq!(doc.front.claude_sessions, ["abc"]);
        assert_eq!(doc.front.worktree.as_deref(), Some(Path::new("/r/.worktrees/x")));
        assert_eq!(doc.front.state, StateWord::Idle);
    }
}
