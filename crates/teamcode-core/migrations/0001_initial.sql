-- 0001_initial (Plan 01). Never edited after merge; later plans add 00NN_<name>.sql files.
-- Ids TEXT, timestamps TEXT (RFC 3339), booleans INTEGER, JSON TEXT.

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    root TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    memory_location TEXT NOT NULL, -- json MemoryLocation
    created_at TEXT NOT NULL
);

CREATE TABLE workspaces (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    file_path TEXT,
    policy TEXT NOT NULL DEFAULT 'ReadAndModifyOnRequest',
    updated_at TEXT NOT NULL
);

CREATE TABLE workspace_projects (
    workspace_id TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects (id),
    "order" INTEGER NOT NULL,
    protected INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (workspace_id, project_id)
);

CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects (id),
    workspace_id TEXT REFERENCES workspaces (id),
    title TEXT NOT NULL,
    stage TEXT NOT NULL,
    state TEXT NOT NULL, -- json AgentState
    priority TEXT NOT NULL,
    flagged INTEGER NOT NULL DEFAULT 0,
    hidden INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    memory_dir TEXT NOT NULL,
    json TEXT NOT NULL, -- the full Task
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    stage_entered_at TEXT NOT NULL
);
CREATE INDEX tasks_project_stage ON tasks (project_id, stage);

CREATE TABLE task_timeline (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    at TEXT NOT NULL,
    kind TEXT NOT NULL, -- entered | handback | commit
    stage TEXT,
    detail TEXT
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    task_id TEXT REFERENCES tasks (id) ON DELETE SET NULL,
    kind TEXT NOT NULL,
    cwd TEXT NOT NULL,
    driver TEXT NOT NULL,
    started_at TEXT NOT NULL,
    exited INTEGER
);

CREATE TABLE views (
    id TEXT PRIMARY KEY,
    container_kind TEXT NOT NULL,
    container_id TEXT NOT NULL,
    name TEXT NOT NULL,
    "order" INTEGER NOT NULL,
    layout TEXT NOT NULL -- json LayoutNode
);

CREATE TABLE settings (
    scope_kind TEXT NOT NULL,
    scope_id TEXT NOT NULL DEFAULT '',
    key TEXT NOT NULL,
    json TEXT NOT NULL,
    PRIMARY KEY (scope_kind, scope_id, key)
);

CREATE TABLE cost_buckets (
    task_id TEXT NOT NULL,
    stage TEXT NOT NULL,
    model TEXT NOT NULL,
    hour TEXT NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd REAL NOT NULL DEFAULT 0,
    estimated INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (task_id, stage, model, hour)
);

-- The Jira token lives in the keychain, never here.
CREATE TABLE jira_links (
    project_id TEXT PRIMARY KEY REFERENCES projects (id) ON DELETE CASCADE,
    site TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE canvas_positions (
    task_id TEXT NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    node_id TEXT NOT NULL,
    x REAL NOT NULL,
    y REAL NOT NULL,
    PRIMARY KEY (task_id, node_id)
);
