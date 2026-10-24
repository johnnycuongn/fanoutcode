//! The cost ledger (§10.6): per-message usage from Claude Code's transcripts, priced with LiveSpec's table, kept in
//! hourly buckets per task, stage, model and agent; plus the live status-line numbers between turns.

pub mod price;
pub mod transcript;
