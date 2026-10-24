//! The SQLite side of the cost ledger: `cost_buckets` (task, stage, model, hour) and its per-agent split
//! `cost_agent_buckets`, written in one transaction; and the transcript cursors.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use chrono::Utc;
use livespec_core::llm::Usage;
use rusqlite::{params, Connection, OptionalExtension};

use super::price::price_usage;
use super::transcript::{parse_transcript, TranscriptCursor, UsageRecord};
use crate::changes::MAIN_AGENT;
use crate::domain::{Stage, TaskId};
use crate::Result;

/// The agent key of headless helper calls in `by_agent`.
pub const HELPER_AGENT: &str = "helpers";

fn hour_of(r: &UsageRecord) -> i64 {
    r.at.timestamp().div_euclid(3600)
}

/// Adds one priced record to both bucket tables. `input_tokens` counts the cache tokens too.
fn add(
    conn: &Connection,
    task_id: &TaskId,
    stage: Stage,
    agent: &str,
    r: &UsageRecord,
    cost: Option<f64>,
) -> Result<()> {
    let input = (r.input + r.cache_write + r.cache_read) as i64;
    let (cost, estimated) = (cost.unwrap_or(0.0), cost.is_none());
    let hour = hour_of(r);
    conn.execute(
        "INSERT INTO cost_buckets (task_id, stage, model, hour, input_tokens, output_tokens, cost_usd, estimated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (task_id, stage, model, hour) DO UPDATE SET
           input_tokens = input_tokens + excluded.input_tokens, output_tokens = output_tokens + excluded.output_tokens,
           cost_usd = cost_usd + excluded.cost_usd, estimated = max(estimated, excluded.estimated)",
        params![
            task_id.as_str(),
            stage.as_str(),
            r.model,
            hour.to_string(),
            input,
            r.output as i64,
            cost,
            estimated
        ],
    )?;
    conn.execute(
        "INSERT INTO cost_agent_buckets (task_id, agent_id, model, hour, input_tokens, output_tokens, cost_usd, estimated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (task_id, agent_id, model, hour) DO UPDATE SET
           input_tokens = input_tokens + excluded.input_tokens, output_tokens = output_tokens + excluded.output_tokens,
           cost_usd = cost_usd + excluded.cost_usd, estimated = max(estimated, excluded.estimated)",
        params![task_id.as_str(), agent, r.model, hour, input, r.output as i64, cost, estimated],
    )?;
    Ok(())
}

/// Transcript records into the task's buckets for `stage`, priced from the table (unpriced: estimated, $0).
pub fn record(conn: &Connection, task_id: &TaskId, stage: Stage, records: &[UsageRecord]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    for r in records {
        let agent = r.agent_id.as_deref().unwrap_or(MAIN_AGENT);
        add(&tx, task_id, stage, agent, r, price_usage(r))?;
    }
    tx.commit()?;
    Ok(())
}

/// One helper call's usage, now, under agent `helpers`: its reported cost, else the table's.
pub fn record_helper(conn: &Connection, task_id: &TaskId, stage: Stage, usage: &Usage) -> Result<()> {
    let r = UsageRecord {
        at: Utc::now(),
        model: usage.model.clone(),
        input: usage.input,
        cache_write: usage.cache_write,
        cache_read: usage.cache_read,
        output: usage.output,
        agent_id: None,
    };
    let cost = usage.cost_usd.filter(|c| c.is_finite()).or_else(|| price_usage(&r));
    let tx = conn.unchecked_transaction()?;
    add(&tx, task_id, stage, HELPER_AGENT, &r, cost)?;
    tx.commit()?;
    Ok(())
}

/// The transcript's cursor; the start when it was never read.
pub fn load_cursor(conn: &Connection, path: &Path) -> TranscriptCursor {
    conn.query_row(
        "SELECT byte_offset, last_message_id FROM transcript_cursors WHERE path = ?1",
        [path.to_string_lossy()],
        |r| {
            Ok(TranscriptCursor {
                byte_offset: r.get::<_, i64>(0)? as u64,
                last_message_id: r.get(1)?,
            })
        },
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or_default()
}

pub fn save_cursor(
    conn: &Connection,
    path: &Path,
    task_id: &TaskId,
    agent_id: &str,
    c: &TranscriptCursor,
) -> Result<()> {
    conn.execute(
        "INSERT INTO transcript_cursors (path, task_id, agent_id, byte_offset, last_message_id) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (path) DO UPDATE SET byte_offset = excluded.byte_offset, last_message_id = excluded.last_message_id",
        params![path.to_string_lossy(), task_id.as_str(), agent_id, c.byte_offset as i64, c.last_message_id],
    )?;
    Ok(())
}

/// Registers a transcript for the task (`""` for the main session) without moving an existing cursor.
pub fn register_transcript(conn: &Connection, path: &Path, task_id: &TaskId, agent_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO transcript_cursors (path, task_id, agent_id, byte_offset, last_message_id)
         VALUES (?1, ?2, ?3, 0, NULL)",
        params![path.to_string_lossy(), task_id.as_str(), agent_id],
    )?;
    Ok(())
}

/// The task's transcripts and their agent ids (`""` for the main session), oldest first.
pub fn transcripts_for(conn: &Connection, task_id: &TaskId) -> Vec<(PathBuf, String)> {
    let Ok(mut stmt) = conn.prepare("SELECT path, agent_id FROM transcript_cursors WHERE task_id = ?1 ORDER BY rowid")
    else {
        return Vec::new();
    };
    stmt.query_map([task_id.as_str()], |r| {
        Ok((PathBuf::from(r.get::<_, String>(0)?), r.get::<_, String>(1)?))
    })
    .map(|rows| rows.flatten().collect())
    .unwrap_or_default()
}

/// Reads the transcript from its cursor, records what is new under `stage`, and saves the cursor. The number of
/// records added; a missing file adds none.
pub fn ingest(conn: &Connection, task_id: &TaskId, stage: Stage, path: &Path, agent_id: &str) -> Result<usize> {
    let cursor = load_cursor(conn, path);
    let mut buf = Vec::new();
    match std::fs::File::open(path) {
        Ok(mut f) => {
            f.seek(SeekFrom::Start(cursor.byte_offset))?;
            f.read_to_end(&mut buf)?;
        }
        Err(_) => return Ok(0),
    }
    let shifted = TranscriptCursor {
        byte_offset: 0,
        last_message_id: cursor.last_message_id.clone(),
    };
    let (mut records, next) = parse_transcript(&buf, &shifted);
    if !agent_id.is_empty() {
        for r in &mut records {
            r.agent_id.get_or_insert_with(|| agent_id.to_owned());
        }
    }
    record(conn, task_id, stage, &records)?;
    let next = TranscriptCursor {
        byte_offset: cursor.byte_offset + next.byte_offset,
        last_message_id: next.last_message_id,
    };
    save_cursor(conn, path, task_id, agent_id, &next)?;
    Ok(records.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{insert_project, insert_task, test_conn};

    #[test]
    fn ingest_reads_each_transcript_line_once() {
        let conn = test_conn();
        insert_project(&conn, "p-1a-1");
        let t = insert_task(&conn, "t-1a-1", "p-1a-1");
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.jsonl");
        std::fs::write(&path, include_bytes!("fixtures/main.jsonl")).unwrap();
        register_transcript(&conn, &path, &t.id, "").unwrap();
        assert_eq!(transcripts_for(&conn, &t.id), vec![(path.clone(), String::new())]);
        assert_eq!(ingest(&conn, &t.id, Stage::Build, &path, "").unwrap(), 2);
        assert_eq!(ingest(&conn, &t.id, Stage::Build, &path, "").unwrap(), 0, "nothing new");
        register_transcript(&conn, &path, &t.id, "").unwrap();
        assert_eq!(
            ingest(&conn, &t.id, Stage::Build, &path, "").unwrap(),
            0,
            "re-registering keeps the cursor"
        );
    }
}
