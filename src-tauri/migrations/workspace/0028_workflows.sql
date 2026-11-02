-- 0.10.11 (D-068). Additive only: workflows and their runs as records of two standard objects,
-- the versions a workflow goes through, and the automated triggers of the active versions.

CREATE TABLE workflow (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL DEFAULT '',
    status               TEXT NOT NULL DEFAULT 'DRAFT',   -- DRAFT · ACTIVE · DEACTIVATED (the engine checks)
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_workflow_deleted ON workflow (deleted_at);

-- A version: the trigger and the steps as one JSON document (Twenty's shape), with its status.
CREATE TABLE workflow_version (
    id              TEXT PRIMARY KEY,
    workflow_id     TEXT NOT NULL REFERENCES workflow (id) ON DELETE CASCADE,
    name            TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'DRAFT',   -- DRAFT · ACTIVE · DEACTIVATED · ARCHIVED (the engine checks)
    definition_json TEXT NOT NULL DEFAULT '{}',
    position        REAL NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_workflow_version_workflow ON workflow_version (workflow_id, position);

-- A run: a record (name, status, when) plus what the engine needs to carry it on after a quit —
-- the trigger's data, the steps' outputs so far, the steps done, a delay's end, a form waited for.
CREATE TABLE workflow_run (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL DEFAULT '',
    status               TEXT NOT NULL DEFAULT 'NOT_STARTED',  -- NOT_STARTED · RUNNING · COMPLETED · FAILED · WAITING
    workflow_id          TEXT REFERENCES workflow (id) ON DELETE CASCADE,
    version_id           TEXT,
    started_at           TEXT,
    ended_at             TEXT,
    trigger_json         TEXT NOT NULL DEFAULT '{}',
    context_json         TEXT NOT NULL DEFAULT '{}',
    done_json            TEXT NOT NULL DEFAULT '[]',
    wait_until           TEXT,
    waiting_for          TEXT NOT NULL DEFAULT '',
    error                TEXT NOT NULL DEFAULT '',
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'WORKFLOW',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_workflow_run_deleted ON workflow_run (deleted_at);
CREATE INDEX idx_workflow_run_workflow ON workflow_run (workflow_id, created_at);
CREATE INDEX idx_workflow_run_waiting ON workflow_run (status, wait_until);

-- The active versions' automated triggers: a record event to watch, or a schedule and its next time.
CREATE TABLE workflow_automated_trigger (
    id            TEXT PRIMARY KEY,
    workflow_id   TEXT NOT NULL REFERENCES workflow (id) ON DELETE CASCADE,
    version_id    TEXT NOT NULL REFERENCES workflow_version (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,                     -- RECORD · SCHEDULE · WEBHOOK (the engine checks)
    settings_json TEXT NOT NULL DEFAULT '{}',
    next_at       TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_workflow_trigger_kind ON workflow_automated_trigger (kind, next_at);
CREATE INDEX idx_workflow_trigger_version ON workflow_automated_trigger (version_id);

-- One ACTIVE and one DRAFT version a workflow (the engine reads them with fetch_optional), and
-- the trigger rows by their workflow (rebuilt on activation).
CREATE UNIQUE INDEX idx_workflow_version_active ON workflow_version (workflow_id) WHERE status = 'ACTIVE';
CREATE UNIQUE INDEX idx_workflow_version_draft ON workflow_version (workflow_id) WHERE status = 'DRAFT';
CREATE INDEX idx_workflow_trigger_workflow ON workflow_automated_trigger (workflow_id);
