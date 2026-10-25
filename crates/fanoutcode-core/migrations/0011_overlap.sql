-- 0011_overlap (Plan 11). Never edited after merge.
ALTER TABLE tasks ADD COLUMN base_branch TEXT;
CREATE TABLE task_overlaps (
    task_id TEXT NOT NULL, other_task TEXT, repo_root TEXT NOT NULL, path TEXT NOT NULL,
    relation TEXT NOT NULL,            -- Claimed | GrantedOnTop | TakenOver | PlannedBy | Committed
    pending_json TEXT,                 -- PendingDecision while a decision is open, else NULL
    held INTEGER NOT NULL DEFAULT 0,   -- the developer chose Hold; pending stays until the claim is released
    noted_out INTEGER NOT NULL DEFAULT 0, noted_in INTEGER NOT NULL DEFAULT 0,
    decided_at TEXT, updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, repo_root, path, other_task));
CREATE TABLE task_footprint (
    task_id TEXT NOT NULL, repo_root TEXT NOT NULL, path TEXT NOT NULL,
    source TEXT NOT NULL,              -- Plan | ChangeSet
    note TEXT, added_at TEXT NOT NULL,
    PRIMARY KEY (task_id, repo_root, path, source));
