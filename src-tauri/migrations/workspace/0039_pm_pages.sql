-- 0.11.7 (D-080): Plane's pages — a project's documents beside its work: a name (empty reads
-- Untitled), an emoji, the editor's document (JSON) with its plain text, private or public,
-- locked, labels, sub-pages; archived with their sub-pages; a version kept as they change;
-- starred in the sidebar's Favorites (below).
--   is_private, is_locked   0 or 1 — flags (one owner: no access rights)
--   created_by, updated_by  who acted ("me", an agent, an automation)

CREATE TABLE pm_page (
    id                TEXT NOT NULL PRIMARY KEY,
    project_id        TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    -- A sub-page's page. Deleting a page takes its sub-pages (in Rust); the key only keeps no
    -- dangling parent.
    parent_id         TEXT REFERENCES pm_page (id) ON DELETE SET NULL,
    name              TEXT NOT NULL DEFAULT '',
    emoji             TEXT NOT NULL DEFAULT '',
    description_json  TEXT,
    description_text  TEXT NOT NULL DEFAULT '',
    is_private        INTEGER NOT NULL DEFAULT 0 CHECK (is_private IN (0, 1)),
    is_locked         INTEGER NOT NULL DEFAULT 0 CHECK (is_locked IN (0, 1)),
    archived_at       TEXT,
    sort_order        REAL NOT NULL DEFAULT 65535,
    created_by        TEXT NOT NULL DEFAULT 'me',
    updated_by        TEXT NOT NULL DEFAULT 'me',
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (parent_id IS NOT id)
);

CREATE INDEX idx_pm_page_project ON pm_page (project_id, sort_order);
CREATE INDEX idx_pm_page_parent ON pm_page (parent_id);

-- A page's labels: the project's (pm_label).
CREATE TABLE pm_page_label (
    page_id     TEXT NOT NULL REFERENCES pm_page (id) ON DELETE CASCADE,
    label_id    TEXT NOT NULL REFERENCES pm_label (id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (page_id, label_id)
);

CREATE INDEX idx_pm_page_label_label ON pm_page_label (label_id);

-- The versions: the name and the document as they were (a restore writes the page from one).
CREATE TABLE pm_page_version (
    id                TEXT NOT NULL PRIMARY KEY,
    page_id           TEXT NOT NULL REFERENCES pm_page (id) ON DELETE CASCADE,
    name              TEXT NOT NULL DEFAULT '',
    description_json  TEXT,
    description_text  TEXT NOT NULL DEFAULT '',
    created_by        TEXT NOT NULL DEFAULT 'me',
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_page_version_page ON pm_page_version (page_id, created_at);

-- The sidebar's Favorites hold a page as they hold a view, a cycle or a module (0036, 0038).
ALTER TABLE favorite ADD COLUMN pm_page_id TEXT REFERENCES pm_page (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_pm_page ON favorite (pm_page_id);
