use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::*;
use crate::changes::{ChangeSource, ChangedFile, RepoChanges, MAIN_AGENT};
use crate::domain::{AgentState, Stage, Task};
use crate::git::FileStatus;
use crate::hooks::parse::parse_line;
use crate::overlap::footprint::{self, FootprintEntry, FootprintRow, FootprintSource};
use crate::overlap::store;
use crate::test_support::{insert_project_at, insert_task, task_fixture, test_conn};

fn now() -> DateTime<Utc> {
    "2026-10-25T10:00:00Z".parse().unwrap()
}

fn reader(files: &[(&str, &str)]) -> impl Fn(&Path) -> Option<String> {
    let map: HashMap<PathBuf, String> = files.iter().map(|(p, c)| (PathBuf::from(p), c.to_string())).collect();
    move |p: &Path| map.get(p).cloned()
}

fn changed(path: &str, status: FileStatus) -> ChangedFile {
    ChangedFile {
        path: path.into(),
        status,
        added: 1,
        removed: 0,
        binary: false,
        owners: vec![MAIN_AGENT.into()],
        shared: false,
        source: ChangeSource::Hook,
    }
}

fn planned(task: &Task, repo: &str, path: &str) -> FootprintRow {
    FootprintRow {
        task_id: task.id.clone(),
        repo_root: repo.into(),
        path: path.into(),
        source: FootprintSource::Plan,
        note: None,
        added_at: now(),
    }
}

#[test]
fn summaries_skip_me_and_archived() {
    let me = task_fixture("t-a1", "p-1a-1");
    let mut gone = task_fixture("t-c3", "p-1a-1");
    gone.archived = true;
    let other = task_fixture("t-b2", "p-1a-1");
    let got = summaries(
        &me,
        &[me.clone(), gone, other],
        |_| Vec::new(),
        &[],
        &HashMap::new(),
        reader(&[]),
    );
    assert_eq!(got.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Task t-b2"]);
    assert_eq!(got[0].intent, "Task t-b2", "no intent.md: the title");
    assert_eq!(got[0].branch, None);
}

#[test]
fn summaries_carry_intent_first_line_planned_touched_added_and_exports() {
    let me = task_fixture("t-a1", "p-1a-1");
    let mut login = task_fixture("t-b2", "p-1a-1");
    login.title = "Login".into();
    login.stage = Stage::Build;
    login.state = AgentState::Working;
    let footprints = [
        planned(&login, "/app", "src/auth/session.ts"),
        planned(&login, "/app", "src/auth/form.tsx"),
        planned(&me, "/app", "src/mine.ts"),
    ];
    let files_of = |id: &crate::domain::TaskId| {
        if id.as_str() != "t-b2" {
            return Vec::new();
        }
        vec![RepoChanges {
            repo_root: "/wt/login".into(),
            base: None,
            files: vec![
                changed("src/auth/session.ts", FileStatus::Modified),
                changed("src/auth/form.tsx", FileStatus::Untracked),
                changed("src/old.ts", FileStatus::Deleted),
            ],
            other: vec![changed("src/not-mine.ts", FileStatus::Modified)],
        }]
    };
    let branches = HashMap::from([(login.id.clone(), "fo/login".to_owned())]);
    let read = reader(&[
        (
            "/m/t-b2/intent.md",
            "\n# Let users sign in with email\n\nMore detail.\n",
        ),
        ("/wt/login/src/auth/session.ts", "export function startSession() {}\n"),
        (
            "/wt/login/src/auth/form.tsx",
            "export default function LoginForm() {}\nexport function startSession() {}\n",
        ),
    ]);
    let got = summaries(&me, &[login], files_of, &footprints, &branches, read);
    assert_eq!(got.len(), 1);
    let s = &got[0];
    assert_eq!(
        (s.title.as_str(), s.stage, &s.state),
        ("Login", Stage::Build, &AgentState::Working)
    );
    assert_eq!(s.branch.as_deref(), Some("fo/login"));
    assert_eq!(s.intent, "Let users sign in with email");
    assert_eq!(s.planned, ["src/auth/session.ts", "src/auth/form.tsx"]);
    assert_eq!(s.touched, ["src/auth/session.ts", "src/auth/form.tsx", "src/old.ts"]);
    assert_eq!(s.added_files, ["src/auth/form.tsx"]);
    assert_eq!(s.exported, ["startSession", "LoginForm"]);
}

#[test]
fn head_branch_reads_a_checkout_and_a_worktree() {
    let read = reader(&[
        ("/app/.git/HEAD", "ref: refs/heads/main\n"),
        ("/wt/login/.git", "gitdir: /app/.git/worktrees/login\n"),
        ("/app/.git/worktrees/login/HEAD", "ref: refs/heads/fo/login\n"),
        ("/wt/rel/.git", "gitdir: ../../app/.git/worktrees/rel\n"),
        ("/wt/rel/../../app/.git/worktrees/rel/HEAD", "ref: refs/heads/fo/rel\n"),
        ("/detached/.git/HEAD", "4b825dc642cb6eb9a060e54bf8d69288fbee4904\n"),
    ]);
    assert_eq!(head_branch(Path::new("/app"), &read).as_deref(), Some("main"));
    assert_eq!(head_branch(Path::new("/wt/login"), &read).as_deref(), Some("fo/login"));
    assert_eq!(head_branch(Path::new("/wt/rel"), &read).as_deref(), Some("fo/rel"));
    assert_eq!(head_branch(Path::new("/detached"), &read), None);
    assert_eq!(head_branch(Path::new("/nowhere"), &read), None);
}

#[test]
fn candidates_are_the_active_tasks_sharing_a_repo() {
    let c = test_conn();
    insert_project_at(&c, "p-1a-1", "/app");
    insert_project_at(&c, "p-1a-2", "/api");
    insert_project_at(&c, "p-1a-3", "/far");
    let mut me = insert_task(&c, "t-a1", "p-1a-1");
    me.linked_roots = vec!["/api".into()];
    crate::db::tasks::update(&c, &me).unwrap();
    insert_task(&c, "t-b2", "p-1a-1");
    insert_task(&c, "t-c3", "p-1a-2");
    insert_task(&c, "t-d4", "p-1a-3");
    let mut links_me = insert_task(&c, "t-e5", "p-1a-3");
    links_me.linked_roots = vec!["/app".into()];
    crate::db::tasks::update(&c, &links_me).unwrap();
    let mut archived = insert_task(&c, "t-f6", "p-1a-1");
    archived.archived = true;
    crate::db::tasks::update(&c, &archived).unwrap();
    let got = candidates(&c, &me).unwrap();
    let ids: Vec<(&str, &Path)> = got.iter().map(|(t, h)| (t.id.as_str(), h.as_path())).collect();
    assert_eq!(
        ids,
        [
            ("t-b2", Path::new("/app")),
            ("t-c3", Path::new("/api")),
            ("t-e5", Path::new("/far"))
        ]
    );
}

#[test]
fn others_section_lists_the_others_from_the_store() {
    let c = test_conn();
    insert_project_at(&c, "p-1a-1", "/app");
    let me = insert_task(&c, "t-a1", "p-1a-1");
    assert_eq!(others_section(&c, &me, |_| Vec::new(), reader(&[])).unwrap(), "");
    let mut login = task_fixture("t-b2", "p-1a-1");
    login.title = "Login".into();
    login.worktree_path = Some("/wt/login".into());
    crate::db::tasks::insert(&c, &login).unwrap();
    footprint::replace_plan(
        &c,
        &login.id,
        Path::new("/app"),
        &[FootprintEntry {
            path: "src/auth/session.ts".into(),
            note: None,
        }],
        now(),
    )
    .unwrap();
    let read = reader(&[
        ("/wt/login/.git", "gitdir: /app/.git/worktrees/login"),
        ("/app/.git/worktrees/login/HEAD", "ref: refs/heads/fo/login"),
    ]);
    let text = others_section(&c, &me, |_| Vec::new(), read).unwrap();
    assert!(text.starts_with("## Others working here"), "{text}");
    for needle in ["### Login", "fo/login", "src/auth/session.ts"] {
        assert!(text.contains(needle), "missing {needle:?} in\n{text}");
    }
}

fn soft_fixture() -> (rusqlite::Connection, Task, Task) {
    let c = test_conn();
    insert_project_at(&c, "p-1a-1", "/app");
    let mut a = task_fixture("t-a1", "p-1a-1");
    a.title = "Checkout".into();
    a.worktree_path = Some("/wt/a".into());
    crate::db::tasks::insert(&c, &a).unwrap();
    let mut b = task_fixture("t-b2", "p-1a-1");
    b.title = "Login".into();
    b.worktree_path = Some("/wt/b".into());
    crate::db::tasks::insert(&c, &b).unwrap();
    let entry = FootprintEntry {
        path: "src/a.ts".into(),
        note: None,
    };
    footprint::replace_plan(&c, &b.id, Path::new("/app"), &[entry], now()).unwrap();
    (c, a, b)
}

#[test]
fn soft_notes_go_once_per_direction_between_separate_trees() {
    let (c, a, b) = soft_fixture();
    let read = reader(&[
        ("/wt/a/.git", "gitdir: /app/.git/worktrees/a"),
        ("/app/.git/worktrees/a/HEAD", "ref: refs/heads/fo/checkout"),
        ("/wt/b/.git", "gitdir: /app/.git/worktrees/b"),
        ("/app/.git/worktrees/b/HEAD", "ref: refs/heads/fo/login"),
    ]);
    let notes = soft_notes(&c, &a.id, Path::new("/wt/a"), "src/a.ts", &read, now()).unwrap();
    assert_eq!(notes.len(), 2, "{notes:?}");
    assert_eq!(notes[0].to, a.id);
    assert_eq!(notes[0].about, b.id);
    assert_eq!(
        notes[0].text,
        "Fanout Code: task Login also changes src/a.ts on fo/login. Keep your change narrow and expect a merge."
    );
    assert_eq!(
        (notes[0].to_tree.as_path(), notes[0].about_tree.as_path()),
        (Path::new("/wt/a"), Path::new("/wt/b"))
    );
    assert_eq!(notes[1].to, b.id);
    assert!(notes[1]
        .text
        .contains("task Checkout also changes src/a.ts on fo/checkout"));
    // both rows now carry the notes, so the next edit (by either side) sends nothing
    let mine = store::list(&c, &a.id).unwrap();
    assert!(mine[0].noted_out && mine[0].noted_in, "{mine:?}");
    let theirs = store::list(&c, &b.id).unwrap();
    assert!(theirs[0].noted_out && theirs[0].noted_in, "{theirs:?}");
    assert!(soft_notes(&c, &a.id, Path::new("/wt/a"), "src/a.ts", &read, now())
        .unwrap()
        .is_empty());
    assert!(soft_notes(&c, &b.id, Path::new("/wt/b"), "src/a.ts", &read, now())
        .unwrap()
        .is_empty());
    // a file outside the other's footprint, and an unknown branch
    assert!(soft_notes(&c, &a.id, Path::new("/wt/a"), "src/z.ts", &read, now())
        .unwrap()
        .is_empty());
}

#[test]
fn soft_notes_skip_a_shared_tree() {
    let (c, mut a, mut b) = soft_fixture();
    a.worktree_path = None;
    b.worktree_path = None;
    crate::db::tasks::update(&c, &a).unwrap();
    crate::db::tasks::update(&c, &b).unwrap();
    assert!(
        soft_notes(&c, &a.id, Path::new("/app"), "src/a.ts", &reader(&[]), now())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn soft_note_without_a_branch_says_another_branch_and_logs_both_sides() {
    let (c, a, b) = soft_fixture();
    let notes = soft_notes(&c, &a.id, Path::new("/wt/a"), "src/a.ts", &reader(&[]), now()).unwrap();
    assert!(notes[0].text.contains("on another branch"), "{}", notes[0].text);
    let lines = notes[0].log_lines(now());
    assert_eq!(lines[0].0, a.id);
    assert_eq!(lines[1].0, b.id);
    let heard = parse_line(&a.id, 1, lines[0].1.trim_end()).unwrap().event;
    assert_eq!(
        heard,
        crate::hooks::HookEvent::OverlapNoted {
            repo: "/wt/a".into(),
            path: "src/a.ts".into(),
            other_task: b.id.clone(),
            direction: "in".into(),
        }
    );
    let told = parse_line(&b.id, 1, lines[1].1.trim_end()).unwrap().event;
    assert_eq!(
        told,
        crate::hooks::HookEvent::OverlapNoted {
            repo: "/wt/b".into(),
            path: "src/a.ts".into(),
            other_task: a.id,
            direction: "out".into(),
        }
    );
    assert!(lines[0].1.ends_with('\n'));
}

#[test]
fn edited_file_names_the_repo_and_the_relative_path() {
    let repos = vec![PathBuf::from("/wt/a"), PathBuf::from("/api")];
    let edit = |tool: &str, key: &str, path: &str| crate::hooks::HookEvent::PostToolUse {
        tool: tool.into(),
        input: serde_json::json!({ key: path }),
        response: serde_json::Value::Null,
    };
    assert_eq!(
        edited_file(&edit("Edit", "file_path", "/wt/a/src/a.ts"), &repos),
        Some((PathBuf::from("/wt/a"), "src/a.ts".to_owned()))
    );
    assert_eq!(
        edited_file(&edit("NotebookEdit", "notebook_path", "/api/n.ipynb"), &repos),
        Some((PathBuf::from("/api"), "n.ipynb".to_owned()))
    );
    assert_eq!(edited_file(&edit("Write", "file_path", "/tmp/x"), &repos), None);
    assert_eq!(edited_file(&edit("Bash", "file_path", "/wt/a/x"), &repos), None);
    assert_eq!(edited_file(&crate::hooks::HookEvent::UserPromptSubmit, &repos), None);
}

#[test]
fn plan_entries_are_relative_to_the_home_repo() {
    let md = "## Files\n\n- src/a.ts — the form\n- ./src/b.ts\n- /wt/a/src/c.ts\n- /app/src/d.ts\n- /etc/passwd\n\
- ../outside.ts\n- src/a.ts\n";
    let got = plan_entries(md, &[PathBuf::from("/wt/a"), PathBuf::from("/app")]);
    let paths: Vec<&str> = got.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["src/a.ts", "src/b.ts", "src/c.ts", "src/d.ts"]);
    assert_eq!(got[0].note.as_deref(), Some("the form"));
    assert!(plan_entries("# Plan\n\nno files section", &[]).is_empty());
}

#[test]
fn new_task_notes_go_to_the_projects_other_active_tasks() {
    let c = test_conn();
    insert_project_at(&c, "p-1a-1", "/app");
    insert_project_at(&c, "p-1a-2", "/api");
    let mut me = task_fixture("t-a1", "p-1a-1");
    me.title = "Login".into();
    me.branch = Some("fo/login".into());
    crate::db::tasks::insert(&c, &me).unwrap();
    insert_task(&c, "t-b2", "p-1a-1");
    insert_task(&c, "t-c3", "p-1a-2");
    let mut gone = insert_task(&c, "t-d4", "p-1a-1");
    gone.archived = true;
    crate::db::tasks::update(&c, &gone).unwrap();
    let notes = new_task_notes(&c, &me, &reader(&[])).unwrap();
    assert_eq!(
        notes,
        [(
            crate::domain::TaskId::parse("t-b2").unwrap(),
            "Fanout Code: task Login has started on fo/login; its planned files: none listed yet".to_owned()
        )]
    );
}
