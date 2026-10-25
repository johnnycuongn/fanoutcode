-- 0010_workspaces (Plan 10). Never edited after merge.
-- The workspace record mirrors its file: the dirty flag (explicit-save mode), the stage prompt fragments and the
-- workspace-level agents. Context docs per project or workspace. The per-project Jira issue cache replaces 0001's
-- placeholder `jira_links` (never written: the site link is the `jira` setting, the token is in the keychain).

ALTER TABLE workspaces ADD COLUMN dirty INTEGER NOT NULL DEFAULT 0;
ALTER TABLE workspaces ADD COLUMN fragments_json TEXT NOT NULL DEFAULT '{}';
ALTER TABLE workspaces ADD COLUMN agents_json TEXT NOT NULL DEFAULT '{}';

CREATE TABLE context_docs (
    id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL, -- project | workspace
    scope_id TEXT NOT NULL,
    kind TEXT NOT NULL, -- File | Folder | Url | Confluence
    target TEXT NOT NULL,
    label TEXT NOT NULL,
    page_id TEXT,
    fetched_at TEXT,
    ord INTEGER NOT NULL
);
CREATE INDEX context_docs_scope ON context_docs (scope_kind, scope_id, ord);

DROP TABLE jira_links;
CREATE TABLE jira_links (
    project_id TEXT NOT NULL,
    issue_key TEXT NOT NULL,
    summary TEXT NOT NULL,
    status TEXT NOT NULL,
    assignee TEXT,
    issue_type TEXT NOT NULL,
    description TEXT NOT NULL,
    url TEXT NOT NULL,
    updated TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    PRIMARY KEY (project_id, issue_key)
);
