-- Registry: the list of workspaces (tenants) and app-wide settings.

CREATE TABLE workspace (
    id             TEXT PRIMARY KEY,              -- UUID v7
    slug           TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    folder_path    TEXT NOT NULL UNIQUE,
    archived       INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    last_opened_at TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_workspace_archived ON workspace (archived);

-- Key/value; the key is the identity, so no separate UUID.
CREATE TABLE setting (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
