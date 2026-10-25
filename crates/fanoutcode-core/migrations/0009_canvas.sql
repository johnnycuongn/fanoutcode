-- Plan 09: saved Team-canvas positions per task (A8). 0001_initial already creates this table (with a cascading
-- foreign key to tasks); this file is the idempotent form the canvas owns, so a database built without it still has
-- the table.
CREATE TABLE IF NOT EXISTS canvas_positions (
  task_id TEXT NOT NULL,
  node_id TEXT NOT NULL,
  x REAL NOT NULL,
  y REAL NOT NULL,
  PRIMARY KEY (task_id, node_id)
);
