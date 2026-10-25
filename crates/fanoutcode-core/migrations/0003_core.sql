-- 0003_core (Plan 03). Never edited after merge.
-- 0001 already holds the task columns the queries need plus the full Task in `json`; this adds A11's two
-- timestamps and the indexes the board and timeline read through.

ALTER TABLE tasks ADD COLUMN state_entered_at TEXT NOT NULL DEFAULT '';
ALTER TABLE tasks ADD COLUMN links_bound_at TEXT;
UPDATE tasks SET state_entered_at = stage_entered_at WHERE state_entered_at = '';

CREATE INDEX IF NOT EXISTS tasks_project ON tasks (project_id, archived);
CREATE INDEX IF NOT EXISTS task_timeline_task ON task_timeline (task_id, id);
CREATE INDEX IF NOT EXISTS views_container ON views (container_kind, container_id, "order");
