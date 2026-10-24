//! Scoped cost summaries (A6, A19): task → that task; project → its non-archived tasks; workspace → the
//! non-archived tasks of its projects; app → every task.

use chrono::{DateTime, Utc};
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};

use crate::settings::Scope;
use crate::Result;

/// One breakdown row (A19); `by_agent` keys are agent ids, `"main"` and `"helpers"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CostLine {
    pub key: String,
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated: bool,
}

/// The status line's running numbers for a task's session, reported beside the ledger (which lags until a Stop).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LiveCost {
    pub cost_usd: f64,
    pub context_used_pct: f32,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct CostSummary {
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub estimated: bool,
    pub live: Option<LiveCost>,
    pub by_project: Vec<CostLine>,
    pub by_task: Vec<CostLine>,
    pub by_stage: Vec<CostLine>,
    pub by_day: Vec<CostLine>,
    pub by_model: Vec<CostLine>,
    pub by_agent: Vec<CostLine>,
}

/// The `task_id` filter for the scope, most specific first, and its parameter.
fn filter(scope: &Scope) -> (&'static str, Vec<String>) {
    if let Some(t) = &scope.task {
        ("b.task_id = ?1", vec![t.as_str().to_owned()])
    } else if let Some(p) = &scope.project {
        (
            "b.task_id IN (SELECT id FROM tasks WHERE project_id = ?1 AND archived = 0)",
            vec![p.as_str().to_owned()],
        )
    } else if let Some(w) = &scope.workspace {
        (
            "b.task_id IN (SELECT id FROM tasks WHERE archived = 0 AND project_id IN \
             (SELECT project_id FROM workspace_projects WHERE workspace_id = ?1))",
            vec![w.as_str().to_owned()],
        )
    } else {
        ("1 = 1", Vec::new())
    }
}

/// `SELECT <key>, sums FROM <table> b WHERE <scope> GROUP BY 1 ORDER BY 1`.
fn lines(conn: &Connection, table: &str, key: &str, scope: &Scope) -> Result<Vec<CostLine>> {
    let (cond, args) = filter(scope);
    let sql = format!(
        "SELECT {key}, coalesce(sum(b.cost_usd), 0), coalesce(sum(b.input_tokens), 0), coalesce(sum(b.output_tokens), 0),
                coalesce(max(b.estimated), 0)
         FROM {table} b WHERE {cond} GROUP BY 1 ORDER BY 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(args.iter()), |r| {
        Ok(CostLine {
            key: r.get::<_, Option<String>>(0)?.unwrap_or_default(),
            cost_usd: r.get(1)?,
            input_tokens: r.get::<_, i64>(2)? as u64,
            output_tokens: r.get::<_, i64>(3)? as u64,
            estimated: r.get::<_, i64>(4)? != 0,
        })
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// The scope's totals and breakdowns; `live` is reported as given (the status line's running numbers).
pub fn summary(conn: &Connection, scope: &Scope, live: Option<&LiveCost>) -> Result<CostSummary> {
    let buckets = "cost_buckets";
    let by_model = lines(conn, buckets, "b.model", scope)?;
    let total = |f: fn(&CostLine) -> f64| by_model.iter().map(f).sum::<f64>();
    Ok(CostSummary {
        cost_usd: total(|l| l.cost_usd),
        input_tokens: by_model.iter().map(|l| l.input_tokens).sum(),
        output_tokens: by_model.iter().map(|l| l.output_tokens).sum(),
        estimated: by_model.iter().any(|l| l.estimated),
        live: live.cloned(),
        by_project: lines(
            conn,
            buckets,
            "(SELECT project_id FROM tasks WHERE tasks.id = b.task_id)",
            scope,
        )?,
        by_task: lines(conn, buckets, "b.task_id", scope)?,
        by_stage: lines(conn, buckets, "b.stage", scope)?,
        by_day: lines(
            conn,
            buckets,
            "date(CAST(b.hour AS INTEGER) * 3600, 'unixepoch')",
            scope,
        )?,
        by_agent: lines(conn, "cost_agent_buckets", "b.agent_id", scope)?,
        by_model,
    })
}
