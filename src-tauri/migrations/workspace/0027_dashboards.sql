-- 0.10.10 (D-067). Additive only: dashboards as records of a standard object, each with a page
-- layout of its own (0023's tables, kind DASHBOARD), pinnable like any record.

CREATE TABLE dashboard (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL DEFAULT '',
    icon                 TEXT NOT NULL DEFAULT 'IconLayoutDashboard',
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_dashboard_deleted ON dashboard (deleted_at);

-- A dashboard's layout goes with the dashboard.
ALTER TABLE page_layout ADD COLUMN dashboard_id TEXT REFERENCES dashboard (id) ON DELETE CASCADE;
CREATE UNIQUE INDEX idx_page_layout_dashboard ON page_layout (dashboard_id) WHERE kind = 'DASHBOARD';

-- A dashboard in the sidebar's Favorites.
ALTER TABLE favorite ADD COLUMN dashboard_id TEXT REFERENCES dashboard (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_dashboard ON favorite (dashboard_id);
