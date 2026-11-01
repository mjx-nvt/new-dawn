-- 0.10.2 (D-061). Additive only: the views of the record engine's objects (Twenty's views).

-- A saved way of looking at an object's records.
--   type           TABLE | KANBAN | CALENDAR (checked in Rust)
--   key            INDEX for the object's default "All <X>" view (never deleted), else NULL
--   open_record_in SIDE_PANEL | RECORD_PAGE
--   visibility     WORKSPACE | UNLISTED (one owner: a flag)
--   group_by_field_id   a SELECT field of the object (table groups)
--   group_sort     MANUAL | ASC | DESC
CREATE TABLE view (
    id                          TEXT PRIMARY KEY,
    object_id                   TEXT NOT NULL REFERENCES object_meta (id) ON DELETE CASCADE,
    name                        TEXT NOT NULL,
    icon                        TEXT NOT NULL DEFAULT '',
    type                        TEXT NOT NULL DEFAULT 'TABLE',
    key                         TEXT,
    position                    REAL NOT NULL DEFAULT 0,
    is_compact                  INTEGER NOT NULL DEFAULT 0 CHECK (is_compact IN (0, 1)),
    open_record_in              TEXT NOT NULL DEFAULT 'SIDE_PANEL',
    visibility                  TEXT NOT NULL DEFAULT 'WORKSPACE',
    group_by_field_id           TEXT REFERENCES field_meta (id) ON DELETE SET NULL,
    group_sort                  TEXT NOT NULL DEFAULT 'MANUAL',
    kanban_field_id             TEXT REFERENCES field_meta (id) ON DELETE SET NULL,
    kanban_aggregate_operation  TEXT,
    kanban_aggregate_field_id   TEXT REFERENCES field_meta (id) ON DELETE SET NULL,
    calendar_field_id           TEXT REFERENCES field_meta (id) ON DELETE SET NULL,
    created_at                  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at                  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_view_object ON view (object_id, position);
CREATE UNIQUE INDEX idx_view_index ON view (object_id) WHERE key = 'INDEX';

-- A column of a view: shown or hidden, in order, its width and its footer aggregate
-- (COUNT, COUNT_EMPTY, COUNT_NOT_EMPTY, COUNT_UNIQUE, PERCENT_EMPTY, PERCENT_NOT_EMPTY, SUM, AVG,
-- MIN, MAX; checked in Rust).
CREATE TABLE view_field (
    id                   TEXT PRIMARY KEY,
    view_id              TEXT NOT NULL REFERENCES view (id) ON DELETE CASCADE,
    field_id             TEXT NOT NULL REFERENCES field_meta (id) ON DELETE CASCADE,
    is_visible           INTEGER NOT NULL DEFAULT 1 CHECK (is_visible IN (0, 1)),
    position             REAL NOT NULL DEFAULT 0,
    size                 INTEGER NOT NULL DEFAULT 180,
    aggregate_operation  TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (view_id, field_id)
);
CREATE INDEX idx_view_field_field ON view_field (field_id);

-- A rule group of a view's filter (Twenty's advanced filter): And / Or / Not, nested.
CREATE TABLE view_filter_group (
    id         TEXT PRIMARY KEY,
    view_id    TEXT NOT NULL REFERENCES view (id) ON DELETE CASCADE,
    parent_id  TEXT REFERENCES view_filter_group (id) ON DELETE CASCADE,
    logical    TEXT NOT NULL DEFAULT 'AND',
    position   REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_view_filter_group_view ON view_filter_group (view_id, position);
CREATE INDEX idx_view_filter_group_parent ON view_filter_group (parent_id);

-- A rule: a field (or a composite's part), the engine's operator and its value; in a group,
-- or at the top level (the view's own AND).
CREATE TABLE view_filter (
    id         TEXT PRIMARY KEY,
    view_id    TEXT NOT NULL REFERENCES view (id) ON DELETE CASCADE,
    field_id   TEXT NOT NULL REFERENCES field_meta (id) ON DELETE CASCADE,
    sub        TEXT NOT NULL DEFAULT '',
    operand    TEXT NOT NULL,
    value_json TEXT NOT NULL DEFAULT 'null',
    group_id   TEXT REFERENCES view_filter_group (id) ON DELETE CASCADE,
    position   REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_view_filter_view ON view_filter (view_id, position);
CREATE INDEX idx_view_filter_field ON view_filter (field_id);
CREATE INDEX idx_view_filter_group_id ON view_filter (group_id);

CREATE TABLE view_sort (
    id         TEXT PRIMARY KEY,
    view_id    TEXT NOT NULL REFERENCES view (id) ON DELETE CASCADE,
    field_id   TEXT NOT NULL REFERENCES field_meta (id) ON DELETE CASCADE,
    sub        TEXT NOT NULL DEFAULT '',
    direction  TEXT NOT NULL DEFAULT 'ASC',
    position   REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_view_sort_view ON view_sort (view_id, position);
CREATE INDEX idx_view_sort_field ON view_sort (field_id);

-- The groups of a grouped table (the select field's option values): order and visibility.
CREATE TABLE view_group (
    id          TEXT PRIMARY KEY,
    view_id     TEXT NOT NULL REFERENCES view (id) ON DELETE CASCADE,
    field_value TEXT NOT NULL DEFAULT '',
    is_visible  INTEGER NOT NULL DEFAULT 1 CHECK (is_visible IN (0, 1)),
    position    REAL NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (view_id, field_value)
);

-- A favorite can be a view (0.10.7 shows the sidebar's Favorites).
ALTER TABLE favorite ADD COLUMN view_id TEXT REFERENCES view (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_view ON favorite (view_id);
