use chrono::{DateTime, Utc};

use super::*;
use crate::test_support::test_conn;

fn at(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}

fn id(s: &str) -> TaskId {
    TaskId::parse(s).unwrap()
}

fn entry(path: &str, note: Option<&str>) -> FootprintEntry {
    FootprintEntry {
        path: path.into(),
        note: note.map(str::to_owned),
    }
}

#[test]
fn parse_files_section_reads_paths_and_notes_and_stops_at_next_heading() {
    let md = "# Plan\n\n## Steps\n- [ ] not a file\n\n## Files\n\n- `src/auth/session.ts` — add refresh\n\
              - src/lib/api.ts\n* docs/a.md —   \n  - `src/nested.ts`—tight note\nprose line\n- \n\n## Risks\n- src/no.ts\n";
    assert_eq!(
        parse_files_section(md),
        vec![
            entry("src/auth/session.ts", Some("add refresh")),
            entry("src/lib/api.ts", None),
            entry("docs/a.md", None),
            entry("src/nested.ts", Some("tight note")),
        ]
    );
    // a top-level heading ends it too, and the heading is matched case-insensitively
    assert_eq!(
        parse_files_section("## files\n- a.rs\n# Next\n- b.rs\n"),
        vec![entry("a.rs", None)]
    );
}

#[test]
fn parse_files_section_is_empty_without_the_heading() {
    assert!(parse_files_section("# Plan\n- src/a.ts\n### Files\n- src/b.ts\n").is_empty());
    assert!(parse_files_section("").is_empty());
}

#[test]
fn replace_plan_replaces_only_plan_rows() {
    let conn = test_conn();
    let (t, repo, now) = (id("t-a1"), Path::new("/r"), at("2026-10-25T10:00:00Z"));
    add_change_set(&conn, &t, repo, &["src/a.ts".into()], now).unwrap();
    replace_plan(
        &conn,
        &t,
        repo,
        &[entry("src/a.ts", Some("n")), entry("src/b.ts", None)],
        now,
    )
    .unwrap();
    replace_plan(&conn, &t, Path::new("/other"), &[entry("x.rs", None)], now).unwrap();
    let later = at("2026-10-25T11:00:00Z");
    replace_plan(&conn, &t, repo, &[entry("src/c.ts", Some("new"))], later).unwrap();
    let rows: Vec<(String, String, FootprintSource, Option<String>)> = footprint(&conn, &t)
        .unwrap()
        .into_iter()
        .map(|r| (r.repo_root.to_string_lossy().into_owned(), r.path, r.source, r.note))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("/other".into(), "x.rs".into(), FootprintSource::Plan, None),
            ("/r".into(), "src/a.ts".into(), FootprintSource::ChangeSet, None),
            (
                "/r".into(),
                "src/c.ts".into(),
                FootprintSource::Plan,
                Some("new".into())
            ),
        ]
    );
}

#[test]
fn add_change_set_keeps_the_first_added_at() {
    let conn = test_conn();
    let (t, repo) = (id("t-a1"), Path::new("/r"));
    add_change_set(&conn, &t, repo, &["a.rs".into()], at("2026-10-25T10:00:00Z")).unwrap();
    add_change_set(
        &conn,
        &t,
        repo,
        &["a.rs".into(), "b.rs".into()],
        at("2026-10-25T11:00:00Z"),
    )
    .unwrap();
    let rows = footprint(&conn, &t).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        FootprintRow {
            task_id: t.clone(),
            repo_root: repo.into(),
            path: "a.rs".into(),
            source: FootprintSource::ChangeSet,
            note: None,
            added_at: at("2026-10-25T10:00:00Z"),
        }
    );
}

#[test]
fn footprints_of_reads_several_tasks() {
    let conn = test_conn();
    let now = at("2026-10-25T10:00:00Z");
    for t in ["t-a1", "t-b2", "t-c3"] {
        add_change_set(&conn, &id(t), Path::new("/r"), &["a.rs".into()], now).unwrap();
    }
    let rows = footprints_of(&conn, &[id("t-c3"), id("t-a1")]).unwrap();
    let tasks: Vec<&str> = rows.iter().map(|r| r.task_id.as_str()).collect();
    assert_eq!(tasks, vec!["t-a1", "t-c3"]);
    assert!(footprints_of(&conn, &[]).unwrap().is_empty());
}

#[test]
fn footprint_source_round_trips() {
    for s in [FootprintSource::Plan, FootprintSource::ChangeSet] {
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(serde_json::from_value::<FootprintSource>(json).unwrap(), s);
    }
    assert_eq!(serde_json::to_value(FootprintSource::ChangeSet).unwrap(), "ChangeSet");
}
