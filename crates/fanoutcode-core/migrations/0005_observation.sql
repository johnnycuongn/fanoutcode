-- 0005_observation (Plan 05). Never edited after merge.
-- Hook-log cursors, the task change set's attributions, linked-project test commands, transcript cursors for the
-- cost ledger, and the per-agent split of cost_buckets.

CREATE TABLE IF NOT EXISTS event_cursors (task_id TEXT PRIMARY KEY, byte_offset INTEGER NOT NULL, seq INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS task_files (task_id TEXT NOT NULL, repo_root TEXT NOT NULL, path TEXT NOT NULL, agent_id TEXT NOT NULL DEFAULT '', source TEXT NOT NULL, added_at TEXT NOT NULL, PRIMARY KEY (task_id, repo_root, path, agent_id));
CREATE TABLE IF NOT EXISTS task_test_commands (task_id TEXT NOT NULL, repo_root TEXT NOT NULL, command TEXT NOT NULL, PRIMARY KEY (task_id, repo_root));
CREATE TABLE IF NOT EXISTS transcript_cursors (path TEXT PRIMARY KEY, task_id TEXT NOT NULL, agent_id TEXT NOT NULL DEFAULT '', byte_offset INTEGER NOT NULL, last_message_id TEXT);
-- per-agent split of cost_buckets for CostSummary.by_agent (overview A6); written beside cost_buckets by cost::ledger
CREATE TABLE IF NOT EXISTS cost_agent_buckets (task_id TEXT NOT NULL, agent_id TEXT NOT NULL, model TEXT NOT NULL, hour INTEGER NOT NULL, input_tokens INTEGER NOT NULL, output_tokens INTEGER NOT NULL, cost_usd REAL NOT NULL, estimated INTEGER NOT NULL, PRIMARY KEY (task_id, agent_id, model, hour));
