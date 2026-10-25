//! Claude Code's session transcript (JSONL): assistant lines carry `message.usage`, repeated on every content-block
//! line of one message (Review Focus 3), so usage is counted once per `message.id`.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::hooks::parse::normalize_agent_id;

/// How far into a transcript the ledger has read, and the last message counted (repeats follow it directly).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranscriptCursor {
    pub byte_offset: u64,
    pub last_message_id: Option<String>,
}

/// One assistant message's usage. `input` excludes the cache counts.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageRecord {
    pub at: DateTime<Utc>,
    pub model: String,
    pub input: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    pub output: u64,
    /// The subagent the line belongs to (`agentId`), normalised; `None` for the main session.
    pub agent_id: Option<String>,
}

fn record_of(v: &Value) -> Option<(String, UsageRecord)> {
    if v.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let msg = v.get("message")?;
    let usage = msg.get("usage")?;
    let id = msg.get("id")?.as_str()?.to_owned();
    let model = msg.get("model").and_then(Value::as_str).unwrap_or("").to_owned();
    let n = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
    let at = v
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map_or_else(Utc::now, |t| t.with_timezone(&Utc));
    Some((
        id,
        UsageRecord {
            at,
            model,
            input: n("input_tokens"),
            cache_write: n("cache_creation_input_tokens"),
            cache_read: n("cache_read_input_tokens"),
            output: n("output_tokens"),
            agent_id: v.get("agentId").and_then(Value::as_str).map(normalize_agent_id),
        },
    ))
}

/// The usage records after `cursor`, and the cursor past the last complete line. One record per `message.id`
/// (a repeat directly after its message is skipped); `<synthetic>` messages are skipped; a partial last line is
/// left for next time.
pub fn parse_transcript(bytes: &[u8], cursor: &TranscriptCursor) -> (Vec<UsageRecord>, TranscriptCursor) {
    let start = (cursor.byte_offset as usize).min(bytes.len());
    let rest = &bytes[start..];
    let Some(end) = rest.iter().rposition(|&b| b == b'\n') else {
        return (Vec::new(), cursor.clone());
    };
    let mut next = TranscriptCursor {
        byte_offset: (start + end + 1) as u64,
        last_message_id: cursor.last_message_id.clone(),
    };
    let mut records = Vec::new();
    for line in rest[..end].split(|&b| b == b'\n') {
        let Ok(v) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let Some((id, rec)) = record_of(&v) else { continue };
        if next.last_message_id.as_deref() == Some(id.as_str()) {
            continue;
        }
        next.last_message_id = Some(id);
        if rec.model == "<synthetic>" {
            continue;
        }
        records.push(rec);
    }
    (records, next)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!("fixtures/main.jsonl");

    #[test]
    fn usage_is_counted_once_per_message_id() {
        let (recs, _) = parse_transcript(FIXTURE, &TranscriptCursor::default());
        assert_eq!(recs.len(), 2);
        assert_eq!(
            (
                recs[0].model.as_str(),
                recs[0].input,
                recs[0].cache_write,
                recs[0].cache_read,
                recs[0].output
            ),
            ("claude-sonnet-5-5", 2, 47212, 28449, 1230)
        );
    }

    #[test]
    fn synthetic_messages_are_skipped() {
        let (recs, _) = parse_transcript(FIXTURE, &TranscriptCursor::default());
        assert!(recs.iter().all(|r| r.model != "<synthetic>"));
    }

    #[test]
    fn a_partial_last_line_is_left_for_next_time() {
        let (_, c) = parse_transcript(FIXTURE, &TranscriptCursor::default());
        assert_eq!(
            c.byte_offset as usize,
            FIXTURE.iter().rposition(|&b| b == b'\n').unwrap() + 1
        );
        assert_eq!(c.last_message_id.as_deref(), Some("msg_3"));
        let (again, _) = parse_transcript(FIXTURE, &c);
        assert!(again.is_empty());
    }

    #[test]
    fn subagent_lines_carry_their_agent_id() {
        let (recs, _) = parse_transcript(FIXTURE, &TranscriptCursor::default());
        assert_eq!(recs[1].agent_id.as_deref(), Some("a1"));
        assert_eq!(recs[0].agent_id, None);
    }
}
