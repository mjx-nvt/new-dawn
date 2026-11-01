-- 0.10.4 (D-062). Additive only: record page layouts (Twenty's page layouts) — per object, tabs of
-- widgets on a 12-column grid. Dashboards (0.10.10) reuse the three tables with kind DASHBOARD.

CREATE TABLE page_layout (
    id          TEXT PRIMARY KEY,
    object_id   TEXT REFERENCES object_meta (id) ON DELETE CASCADE,  -- NULL for a dashboard
    name        TEXT NOT NULL DEFAULT '',
    kind        TEXT NOT NULL DEFAULT 'RECORD',                       -- RECORD | DASHBOARD (checked in Rust)
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE UNIQUE INDEX idx_page_layout_record ON page_layout (object_id) WHERE kind = 'RECORD';
CREATE INDEX idx_page_layout_object ON page_layout (object_id);

CREATE TABLE page_layout_tab (
    id          TEXT PRIMARY KEY,
    layout_id   TEXT NOT NULL REFERENCES page_layout (id) ON DELETE CASCADE,
    title       TEXT NOT NULL DEFAULT '',
    position    REAL NOT NULL DEFAULT 0,
    is_default  INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_page_layout_tab_layout ON page_layout_tab (layout_id, position);

-- A widget on a tab: its type (FIELDS, RELATED, TIMELINE, TASKS, NOTES, FILES, RICH_TEXT, CHART,
-- EMAILS, CALENDAR, IFRAME; New Dawn's COMMITMENTS and MEETINGS — checked in Rust), its place
-- on the grid (12 columns, rows of 56 pt) and its settings (the fields shown, the relation, the
-- text…) as JSON, genuinely nested.
CREATE TABLE page_layout_widget (
    id          TEXT PRIMARY KEY,
    tab_id      TEXT NOT NULL REFERENCES page_layout_tab (id) ON DELETE CASCADE,
    type        TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    position    REAL NOT NULL DEFAULT 0,
    grid_x      INTEGER NOT NULL DEFAULT 0,
    grid_y      INTEGER NOT NULL DEFAULT 0,
    grid_w      INTEGER NOT NULL DEFAULT 12,
    grid_h      INTEGER NOT NULL DEFAULT 6,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_page_layout_widget_tab ON page_layout_widget (tab_id, position);
