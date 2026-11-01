-- 0.10.7 (D-064). Additive only: the sidebar's favorites in folders, the Workspace section's
-- object order and hidden objects, and the recent visits ⌘K lists.

CREATE TABLE favorite_folder (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL DEFAULT '',
    position    REAL NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_favorite_folder_position ON favorite_folder (position);

-- A favorite in a folder; the folder's deletion sends its favorites back to the root.
ALTER TABLE favorite ADD COLUMN folder_id TEXT REFERENCES favorite_folder (id) ON DELETE SET NULL;
CREATE INDEX idx_favorite_folder_id ON favorite (folder_id);

-- One row per record object: its place in the sidebar's Workspace section, whether it is
-- hidden there (Settings → Data model shows it again), and the folder it sits in (a name
-- shared by the items of one folder; '' = none).
CREATE TABLE navigation_item (
    id          TEXT PRIMARY KEY,
    object_id   TEXT NOT NULL UNIQUE REFERENCES object_meta (id) ON DELETE CASCADE,
    position    REAL NOT NULL DEFAULT 0,
    is_hidden   INTEGER NOT NULL DEFAULT 0 CHECK (is_hidden IN (0, 1)),
    folder      TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_navigation_item_position ON navigation_item (position);

-- The records opened lately (object name + id; the row is kept, not the record): ⌘K's Recents.
CREATE TABLE recent_visit (
    id          TEXT PRIMARY KEY,
    object      TEXT NOT NULL,
    record_id   TEXT NOT NULL,
    visited_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (object, record_id)
);
CREATE INDEX idx_recent_visit_at ON recent_visit (visited_at);
