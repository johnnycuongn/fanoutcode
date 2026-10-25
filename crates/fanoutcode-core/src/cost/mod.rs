//! The cost ledger (§10.6): per-message usage from Claude Code's transcripts, priced with LiveSpec's table, kept in
//! hourly buckets per task, stage, model and agent; plus the live status-line numbers between turns.

pub mod ledger;
pub mod price;
pub mod summary;
pub mod transcript;

pub use summary::{summary, CostLine, CostSummary, LiveCost};

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use livespec_core::llm::Usage;
    use rusqlite::Connection;
    use serde_json::json;

    use super::ledger::{record, record_helper};
    use super::transcript::UsageRecord;
    use super::*;
    use crate::domain::{ProjectId, Stage, TaskId};
    use crate::settings::Scope;
    use crate::test_support::{insert_project, task_fixture, test_conn};

    impl Scope {
        fn task(id: &str) -> Scope {
            Scope {
                task: Some(TaskId::parse(id).unwrap()),
                ..Scope::default()
            }
        }
        fn project(id: &str) -> Scope {
            Scope {
                project: Some(ProjectId::parse(id).unwrap()),
                ..Scope::default()
            }
        }
    }

    fn tid(id: &str) -> TaskId {
        TaskId::parse(id).unwrap()
    }

    fn add_task(conn: &Connection, id: &str, project: &str, archived: bool) {
        if crate::db::projects::get(conn, &ProjectId::parse(project).unwrap())
            .unwrap()
            .is_none()
        {
            insert_project(conn, project);
        }
        let mut t = task_fixture(id, project);
        t.archived = archived;
        crate::db::tasks::insert(conn, &t).unwrap();
    }

    fn fresh_db_with_task(id: &str, project: &str, archived: bool) -> Connection {
        let conn = test_conn();
        add_task(&conn, id, project, archived);
        conn
    }

    fn rec_at(at: &str, model: &str, input: u64, cache_write: u64, cache_read: u64, output: u64) -> UsageRecord {
        UsageRecord {
            at: at.parse::<DateTime<Utc>>().unwrap(),
            model: model.into(),
            input,
            cache_write,
            cache_read,
            output,
            agent_id: None,
        }
    }

    #[test]
    fn records_upsert_into_hourly_buckets() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        record(
            &conn,
            &tid("t-1"),
            Stage::Build,
            &[
                rec_at("2026-10-24T05:10:00Z", "claude-sonnet-5-5", 1000, 0, 0, 100),
                rec_at("2026-10-24T05:40:00Z", "claude-sonnet-5-5", 1000, 0, 0, 100),
            ],
        )
        .unwrap();
        let n: i64 = conn
            .query_row("SELECT count(*) FROM cost_buckets", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
        let s = summary(&conn, &Scope::task("t-1"), None).unwrap();
        assert_eq!((s.input_tokens, s.output_tokens, s.estimated), (2000, 200, false));
        assert!((s.cost_usd - 0.006).abs() < 1e-9);
    }

    #[test]
    fn project_scope_excludes_archived_tasks() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        add_task(&conn, "t-2", "p-1", true);
        for t in ["t-1", "t-2"] {
            record(
                &conn,
                &tid(t),
                Stage::Build,
                &[rec_at("2026-10-24T05:10:00Z", "claude-sonnet-5-5", 1000, 0, 0, 0)],
            )
            .unwrap();
        }
        assert_eq!(summary(&conn, &Scope::project("p-1"), None).unwrap().input_tokens, 1000);
        assert_eq!(summary(&conn, &Scope::app(), None).unwrap().input_tokens, 2000);
    }

    #[test]
    fn estimated_flags_propagate() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        record(
            &conn,
            &tid("t-1"),
            Stage::Plan,
            &[rec_at("2026-10-24T05:10:00Z", "mystery-9", 10, 0, 0, 10)],
        )
        .unwrap();
        let s = summary(&conn, &Scope::task("t-1"), None).unwrap();
        assert!(s.estimated);
        assert_eq!(
            s.by_stage,
            vec![CostLine {
                key: "plan".into(),
                cost_usd: 0.0,
                input_tokens: 10,
                output_tokens: 10,
                estimated: true
            }]
        );
        assert_eq!(
            serde_json::to_value(&s.by_stage[0]).unwrap(), // A16/A19: the names Plans 06, 09, 10 read
            json!({"key": "plan", "cost_usd": 0.0, "input_tokens": 10, "output_tokens": 10, "estimated": true})
        );
    }

    #[test]
    fn helper_usage_lands_in_the_tasks_current_stage() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        record_helper(
            &conn,
            &tid("t-1"),
            Stage::Commit,
            &Usage {
                model: "claude-haiku-5-5".into(),
                input: 300,
                cache_write: 0,
                cache_read: 0,
                output: 20,
                cost_usd: Some(0.00004),
            },
        )
        .unwrap();
        let s = summary(&conn, &Scope::task("t-1"), None).unwrap();
        assert_eq!(s.by_stage[0].key, "commit");
        assert!((s.cost_usd - 0.00004).abs() < 1e-12);
    }

    #[test]
    fn summary_breaks_down_by_day_and_model() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        record(
            &conn,
            &tid("t-1"),
            Stage::Build,
            &[
                rec_at("2026-10-23T23:30:00Z", "claude-sonnet-5-5", 1, 0, 0, 0),
                rec_at("2026-10-24T00:30:00Z", "claude-haiku-5-5", 1, 0, 0, 0),
            ],
        )
        .unwrap();
        let s = summary(&conn, &Scope::task("t-1"), None).unwrap();
        assert_eq!(
            s.by_day.iter().map(|l| l.key.as_str()).collect::<Vec<_>>(),
            vec!["2026-10-23", "2026-10-24"]
        );
        assert_eq!(s.by_model.len(), 2);
    }

    #[test]
    fn summary_breaks_down_by_agent() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        let mut sub = rec_at("2026-10-24T05:10:00Z", "claude-haiku-5-5", 10, 0, 0, 1);
        sub.agent_id = Some("a1".into());
        record(
            &conn,
            &tid("t-1"),
            Stage::Build,
            &[rec_at("2026-10-24T05:10:00Z", "claude-sonnet-5-5", 100, 0, 0, 10), sub],
        )
        .unwrap();
        record_helper(
            &conn,
            &tid("t-1"),
            Stage::Build,
            &Usage {
                model: "claude-haiku-5-5".into(),
                input: 5,
                cache_write: 0,
                cache_read: 0,
                output: 1,
                cost_usd: None,
            },
        )
        .unwrap();
        let s = summary(&conn, &Scope::task("t-1"), None).unwrap();
        assert_eq!(
            s.by_agent
                .iter()
                .map(|l| (l.key.as_str(), l.input_tokens))
                .collect::<Vec<_>>(),
            vec![("a1", 10), ("helpers", 5), ("main", 100)]
        );
    }

    #[test]
    fn live_status_line_is_reported_separately() {
        let conn = fresh_db_with_task("t-1", "p-1", false);
        let live = LiveCost {
            cost_usd: 0.42,
            context_used_pct: 37.0,
            at: Utc::now(),
        };
        let s = summary(&conn, &Scope::task("t-1"), Some(&live)).unwrap();
        assert_eq!(s.cost_usd, 0.0);
        assert_eq!(s.live.unwrap().cost_usd, 0.42);
    }
}
