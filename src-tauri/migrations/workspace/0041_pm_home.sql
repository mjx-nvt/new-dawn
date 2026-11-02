-- 0.11.9 (D-082): Plane's Home and navigation — the owner's stickies and quick links, the Home's
-- widgets (on or off, in order), the Projects section's items shown or hidden (Customize
-- navigation), and what was opened lately (Recents).
--   pm_home_widget.key   quick_links | recents | stickies (checked in Rust; no row = the default)
--   pm_navigation.key    home | your_work | inbox | drafts | views | active_cycles | archives
--   pm_recent.kind       item | page | project

CREATE TABLE pm_sticky (
    id                TEXT NOT NULL PRIMARY KEY,
    description_json  TEXT,
    description_text  TEXT NOT NULL DEFAULT '',
    color             TEXT NOT NULL DEFAULT 'yellow',
    sort_order        REAL NOT NULL DEFAULT 65535,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_sticky_order ON pm_sticky (sort_order);

CREATE TABLE pm_quick_link (
    id          TEXT NOT NULL PRIMARY KEY,
    title       TEXT NOT NULL DEFAULT '',
    url         TEXT NOT NULL,
    sort_order  REAL NOT NULL DEFAULT 65535,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- The same address once.
CREATE UNIQUE INDEX idx_pm_quick_link_url ON pm_quick_link (url);

CREATE TABLE pm_home_widget (
    key         TEXT NOT NULL PRIMARY KEY,
    is_enabled  INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0, 1)),
    position    REAL NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE pm_navigation (
    key         TEXT NOT NULL PRIMARY KEY,
    is_hidden   INTEGER NOT NULL DEFAULT 0 CHECK (is_hidden IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- One row a thing opened, its time moved on each visit; read joined to what still exists.
CREATE TABLE pm_recent (
    kind        TEXT NOT NULL,
    ref_id      TEXT NOT NULL,
    project_id  TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    visited_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (kind, ref_id)
);

CREATE INDEX idx_pm_recent_visited ON pm_recent (visited_at);
CREATE INDEX idx_pm_recent_project ON pm_recent (project_id);
