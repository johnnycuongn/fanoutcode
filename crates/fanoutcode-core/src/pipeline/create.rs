//! Creating a task (§6.1): its memory folder with `intent.md`, its row with the project's defaults, and the first
//! two timeline entries, so the row, the timeline and `index.md` exist together.

use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;

use super::{apply, Effect};
use crate::db;
use crate::domain::{AgentState, Priority, ProjectId, Stage, Task, TaskId, WorkspaceId, WorktreeChoice};
use crate::helpers::HelperKind;
use crate::memory::{root_for, slug::title_from_intent, IndexEntry, MemoryFile, TaskFolder, TimelineKind};
use crate::settings::{keys, resolve, Scope};
use crate::{Error, Result};

/// What the new-task sheet sends.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTask {
    pub project_id: ProjectId,
    pub workspace_id: Option<WorkspaceId>,
    pub intent: String,
    pub jira_key: Option<String>,
    /// The Jira summary when created from an issue; else derived from the intent.
    pub title: Option<String>,
    pub now: DateTime<Utc>,
    /// The local date, for the folder name; the shell passes it.
    pub date: NaiveDate,
}

/// The new task; `external` is `[RunHelper(Naming)]` when the title was derived, `[]` when it was given.
#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub task: Task,
    pub external: Vec<Effect>,
}

/// Validates the home project (a member of the workspace, when there is one), makes the folder and `intent.md`,
/// inserts the row with the project's worktree and agent defaults, and records `created` and `entered` at Spec.
/// Linked roots and `links_bound_at` stay empty until the first session starts (Plan 04).
pub fn create_task(conn: &mut Connection, app_data: &Path, new: NewTask) -> Result<Created> {
    let project = db::projects::get(conn, &new.project_id)?.ok_or_else(|| Error::Invalid("Unknown project".into()))?;
    if let Some(ws) = &new.workspace_id {
        if !db::tasks::workspace_contains(conn, ws, &project.id)? {
            return Err(Error::Invalid("Project is not in this workspace".into()));
        }
    }
    let given = new
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned);
    let derived = given.is_none();
    let title = given.unwrap_or_else(|| title_from_intent(&new.intent));
    let scope = Scope {
        workspace: new.workspace_id.clone(),
        project: Some(project.id.clone()),
        task: None,
    };
    let values = db::settings::load(conn, &scope)?;
    let folder = TaskFolder::create(&root_for(&project, app_data), new.date, &title, &new.intent)?;
    // the spec starts as the description the developer just wrote, so the Spec stage never opens empty
    folder.write(MemoryFile::Spec, &format!("{}\n", new.intent.trim()))?;
    let now = new.now;
    let task = Task {
        id: TaskId::new(),
        project_id: project.id.clone(),
        workspace_id: new.workspace_id,
        title,
        stage: Stage::Spec,
        state: AgentState::Idle,
        priority: Priority::None,
        color: None,
        flagged: false,
        hidden: false,
        archived: false,
        jira_key: new.jira_key,
        agent_kind: resolve::<keys::AgentKind>(&values, &scope),
        worktree: match resolve::<keys::Worktree>(&values, &scope) {
            // parallel tasks get their own worktree by default; a folder that cannot host one keeps the checkout
            WorktreeChoice::On if !crate::worktree::can_host(&project.root) => WorktreeChoice::Off,
            w => w,
        },
        worktree_path: None,
        branch: None,
        memory_dir: folder.dir().to_owned(),
        primary_session: None,
        claude_session_id: None,
        linked_roots: Vec::new(),
        base: Vec::new(),
        created_at: now,
        updated_at: now,
        stage_entered_at: now,
        state_entered_at: now,
        links_bound_at: None,
        base_branch: None,
    };
    if let Err(e) = db::tasks::insert(conn, &task) {
        // the folder was just made and holds only intent.md; do not leave an orphan
        let _ = std::fs::remove_dir_all(folder.dir());
        return Err(e);
    }
    let entry = |kind| {
        Effect::WriteIndex(IndexEntry {
            at: now,
            stage: Stage::Spec,
            kind,
        })
    };
    let applied = apply(
        conn,
        &folder,
        &task,
        vec![entry(TimelineKind::Created), entry(TimelineKind::Entered)],
        now,
    )?;
    let external = if derived {
        vec![Effect::RunHelper(HelperKind::Naming)]
    } else {
        Vec::new()
    };
    Ok(Created {
        task: applied.task,
        external,
    })
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::domain::{AgentKind, MemoryLocation};
    use crate::memory::MemoryFile;
    use crate::settings::ScopeKind;
    use crate::test_support::{insert_project, insert_project_at, insert_workspace_with, project_fixture, test_conn};

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    #[test]
    fn create_writes_folder_intent_index_and_row() {
        let mut c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let created = create_task(
            &mut c,
            data.path(),
            NewTask {
                project_id: p.id.clone(),
                workspace_id: None,
                intent: "Add dark mode\n\nUse the theme tokens.".into(),
                jira_key: None,
                title: None,
                now: Utc::now(),
                date: day(),
            },
        )
        .unwrap();
        let t = &created.task;
        assert_eq!(t.title, "Add dark mode");
        assert_eq!(
            t.memory_dir,
            data.path().join("projects/p-1a-1/tasks/2026-10-24-add-dark-mode")
        );
        assert_eq!(
            (t.stage, &t.state, t.worktree, t.agent_kind),
            (
                Stage::Spec,
                &AgentState::Idle,
                WorktreeChoice::Off,
                AgentKind::ClaudeCode
            )
        );
        assert!(t.links_bound_at.is_none() && t.state_entered_at == t.created_at && t.stage_entered_at == t.created_at);
        let f = TaskFolder::open(t.memory_dir.clone());
        assert_eq!(
            f.read(MemoryFile::Intent).unwrap().as_deref(),
            Some("Add dark mode\n\nUse the theme tokens.")
        );
        assert_eq!(
            f.read(MemoryFile::Spec).unwrap().as_deref(),
            Some("Add dark mode\n\nUse the theme tokens.\n"),
            "the spec is seeded from the description"
        );
        assert!(f.read(MemoryFile::Index).unwrap().unwrap().contains("· spec · created"));
        assert_eq!(db::tasks::get(&c, &t.id).unwrap().as_ref(), Some(t));
        assert_eq!(created.external, [Effect::RunHelper(HelperKind::Naming)]);
    }

    #[test]
    fn project_defaults_set_worktree_and_agent_kind_and_a_given_title_skips_naming() {
        let mut c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let repo = crate::git::test_support::temp_repo();
        let p = insert_project_at(&c, "p-1a-1", repo.path().to_str().unwrap());
        db::settings::set(&c, ScopeKind::Project, "p-1a-1", "worktree", &serde_json::json!("On")).unwrap();
        db::settings::set(
            &c,
            ScopeKind::Project,
            "p-1a-1",
            "agent_kind",
            &serde_json::json!("Codex"),
        )
        .unwrap();
        let created = create_task(
            &mut c,
            data.path(),
            NewTask {
                project_id: p.id,
                workspace_id: None,
                intent: "x".into(),
                jira_key: Some("TC-7".into()),
                title: Some("Board filters".into()),
                now: Utc::now(),
                date: day(),
            },
        )
        .unwrap();
        assert_eq!(
            (
                created.task.worktree,
                created.task.agent_kind,
                created.task.jira_key.as_deref(),
                created.task.title.as_str()
            ),
            (WorktreeChoice::On, AgentKind::Codex, Some("TC-7"), "Board filters")
        );
        assert!(created.external.is_empty());
    }

    #[test]
    fn home_project_must_belong_to_the_workspace() {
        let mut c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        insert_project(&c, "p-1a-2");
        insert_workspace_with(&c, "ws-1a-1", &["p-1a-2"]);
        let ws = WorkspaceId::parse("ws-1a-1").unwrap();
        let err = create_task(
            &mut c,
            data.path(),
            NewTask {
                project_id: p.id,
                workspace_id: Some(ws),
                intent: "x".into(),
                jira_key: None,
                title: None,
                now: Utc::now(),
                date: day(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m.contains("not in this workspace")));
    }

    #[test]
    fn two_tasks_with_the_same_title_on_the_same_day_get_distinct_folders() {
        let mut c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let p = insert_project(&c, "p-1a-1");
        let mk = |c: &mut Connection| {
            create_task(
                c,
                data.path(),
                NewTask {
                    project_id: p.id.clone(),
                    workspace_id: None,
                    intent: "Same".into(),
                    jira_key: None,
                    title: None,
                    now: Utc::now(),
                    date: day(),
                },
            )
            .unwrap()
            .task
            .memory_dir
        };
        let a = mk(&mut c);
        let b = mk(&mut c);
        assert_ne!(a, b);
        assert!(b.ends_with("2026-10-24-same-2"));
    }

    #[test]
    fn memory_location_inside_the_project_is_honoured() {
        let mut c = test_conn();
        let data = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let p = project_fixture(
            "p-1a-1",
            MemoryLocation::Folder {
                path: repo.path().join("docs/tasks"),
            },
        );
        db::projects::insert(&c, &p).unwrap();
        let created = create_task(
            &mut c,
            data.path(),
            NewTask {
                project_id: p.id.clone(),
                workspace_id: None,
                intent: "In repo".into(),
                jira_key: None,
                title: None,
                now: Utc::now(),
                date: day(),
            },
        )
        .unwrap();
        assert!(created.task.memory_dir.starts_with(repo.path().join("docs/tasks")));
    }
}
