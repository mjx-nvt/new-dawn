-- 0.11.6 (D-079): Plane's modules — a project's larger pieces of work: a name unique in the
-- project, a description, optional dates, a status, a lead and members, links; an item in several
-- modules; archived when completed or cancelled; starred in the sidebar's Favorites (below).
-- Enumerations are checked in Rust.
--   status          backlog | planned | in_progress | paused | completed | cancelled
--   lead, who       "me" (the owner) or a person's id
--   created_by      who acted ("me", an agent, an automation)

CREATE TABLE pm_module (
    id           TEXT NOT NULL PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    start_date   TEXT,
    target_date  TEXT,
    status       TEXT NOT NULL DEFAULT 'backlog',
    lead         TEXT,
    archived_at  TEXT,
    sort_order   REAL NOT NULL DEFAULT 65535,
    created_by   TEXT NOT NULL DEFAULT 'me',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- Days as YYYY-MM-DD (their text order is their order); the start not after the target.
    CHECK (date(start_date) IS start_date AND date(target_date) IS target_date),
    CHECK (start_date IS NULL OR target_date IS NULL OR start_date <= target_date)
);

-- Names unique a project (ASCII case here; Rust compares them in full, NFC, any case).
CREATE UNIQUE INDEX idx_pm_module_name ON pm_module (project_id, lower(name));
CREATE INDEX idx_pm_module_project ON pm_module (project_id, sort_order);

-- An item in several modules.
CREATE TABLE pm_module_item (
    module_id   TEXT NOT NULL REFERENCES pm_module (id) ON DELETE CASCADE,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (module_id, item_id)
);

CREATE INDEX idx_pm_module_item_item ON pm_module_item (item_id);

CREATE TABLE pm_module_member (
    module_id   TEXT NOT NULL REFERENCES pm_module (id) ON DELETE CASCADE,
    who         TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (module_id, who)
);

CREATE INDEX idx_pm_module_member_who ON pm_module_member (who);

CREATE TABLE pm_module_link (
    id          TEXT NOT NULL PRIMARY KEY,
    module_id   TEXT NOT NULL REFERENCES pm_module (id) ON DELETE CASCADE,
    url         TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_pm_module_link_url ON pm_module_link (module_id, url);

-- A person deleted for good leads and joins no module.
CREATE TRIGGER person_ad_pm_module AFTER DELETE ON person BEGIN
    UPDATE pm_module SET lead = NULL WHERE lead = old.id;
    DELETE FROM pm_module_member WHERE who = old.id;
END;

-- The sidebar's Favorites hold cycles and modules as they hold views (0036): a star is a
-- favorite row. Cycles starred under 0.11.5 (pm_cycle.is_favorite, kept as it was, read no
-- more) become favorites, in the order they were made — not archived ones (archiving unstars).
ALTER TABLE favorite ADD COLUMN pm_cycle_id TEXT REFERENCES pm_cycle (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_pm_cycle ON favorite (pm_cycle_id);
ALTER TABLE favorite ADD COLUMN pm_module_id TEXT REFERENCES pm_module (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_pm_module ON favorite (pm_module_id);

INSERT INTO favorite (id, position, workspace_member_id, pm_cycle_id)
SELECT lower(substr(t.ms, 1, 8) || '-' || substr(t.ms, 9, 4) || '-7' || substr(hex(randomblob(2)), 2) || '-'
             || substr('89ab', 1 + (random() & 3), 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6))),
       (SELECT coalesce(max(position), 0.0) FROM favorite) + row_number() OVER (ORDER BY c.created_at, c.id),
       (SELECT id FROM workspace_member WHERE deleted_at IS NULL ORDER BY created_at LIMIT 1),
       c.id
  FROM pm_cycle c,
       (SELECT printf('%012x', CAST(strftime('%s', 'now') AS INTEGER) * 1000
                               + CAST(substr(strftime('%f', 'now'), 4) AS INTEGER)) AS ms) t
 WHERE c.is_favorite = 1 AND c.archived_at IS NULL;
