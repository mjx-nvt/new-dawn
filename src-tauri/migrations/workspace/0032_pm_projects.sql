-- 0.11.0 (D-073): Plane's projects beside New Dawn's dossier (`project`, D-054). Each project
-- gets one pm_project — Plane's identifier, description, logo, cover, network, lead and
-- defaults, features and automations — with its states, labels and estimates. Enumerations are
-- checked in Rust. Projects made before this release get theirs when the workspace opens.
--   identifier        up to 10 letters or digits, upper case, unique in the workspace (archived
--                     projects keep theirs: their work items are named by it)
--   network           private | public — a label only: nothing is published
--   lead, default_assignee, subscribers_json
--                     "me" (the owner) or a person's id; subscribers a JSON array of them
--   cycles … intake   the features, 0 / 1
--   archive_in        months after an item is completed or cancelled; 0 = off
--   close_in          months without activity; 0 = off; close_state_id the cancelled state
--                     (none: the project's first cancelled state)
--   Cycles, Modules, Views and Pages start on, Intake off (Plane's defaults).
CREATE TABLE pm_project (
    project_id        TEXT NOT NULL PRIMARY KEY REFERENCES project (id) ON DELETE CASCADE,
    identifier        TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',
    emoji             TEXT NOT NULL DEFAULT '',
    cover             TEXT NOT NULL DEFAULT '',
    network           TEXT NOT NULL DEFAULT 'private',
    lead              TEXT,
    default_assignee  TEXT,
    subscribers_json  TEXT NOT NULL DEFAULT '[]',
    time_zone         TEXT NOT NULL DEFAULT '',
    cycles            INTEGER NOT NULL DEFAULT 1 CHECK (cycles IN (0, 1)),
    modules           INTEGER NOT NULL DEFAULT 1 CHECK (modules IN (0, 1)),
    views             INTEGER NOT NULL DEFAULT 1 CHECK (views IN (0, 1)),
    pages             INTEGER NOT NULL DEFAULT 1 CHECK (pages IN (0, 1)),
    intake            INTEGER NOT NULL DEFAULT 0 CHECK (intake IN (0, 1)),
    archive_in        INTEGER NOT NULL DEFAULT 0,
    close_in          INTEGER NOT NULL DEFAULT 0,
    close_state_id    TEXT REFERENCES pm_state (id) ON DELETE SET NULL,
    due_reminders     INTEGER NOT NULL DEFAULT 1 CHECK (due_reminders IN (0, 1)),
    estimate_id       TEXT REFERENCES pm_estimate (id) ON DELETE SET NULL,
    favorite          INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1)),
    sort_order        REAL NOT NULL DEFAULT 65535,
    archived_at       TEXT,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_pm_project_identifier ON pm_project (upper(identifier));

-- A project's states, in five fixed groups (backlog | unstarted | started | completed |
-- cancelled; triage for Intake), each group never empty; one is the default.
CREATE TABLE pm_state (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    color        TEXT NOT NULL DEFAULT '#60646C',
    state_group  TEXT NOT NULL,
    sequence     REAL NOT NULL DEFAULT 65535,
    is_default   INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_state_project ON pm_state (project_id, sequence);
CREATE UNIQUE INDEX idx_pm_state_name ON pm_state (project_id, lower(name));
-- One default a project (Rust clears the old one, then sets the new, in one transaction).
CREATE UNIQUE INDEX idx_pm_state_default ON pm_state (project_id) WHERE is_default = 1;

-- Labels; a label with a parent is in that group (one level).
CREATE TABLE pm_label (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    parent_id    TEXT REFERENCES pm_label (id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    color        TEXT NOT NULL DEFAULT '#60646C',
    sort_order   REAL NOT NULL DEFAULT 65535,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_label_project ON pm_label (project_id, sort_order);
CREATE INDEX idx_pm_label_parent ON pm_label (parent_id);
-- Names unique a project (ASCII case here; Rust compares them in full, NFC, any case).
CREATE UNIQUE INDEX idx_pm_label_name ON pm_label (project_id, lower(name));

-- Estimate systems: points or categories, up to six values; the project's active one is
-- pm_project.estimate_id, the ones it replaced are archived.
CREATE TABLE pm_estimate (
    id           TEXT PRIMARY KEY,
    project_id   TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    kind         TEXT NOT NULL,
    archived_at  TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_estimate_project ON pm_estimate (project_id);

CREATE TABLE pm_estimate_point (
    id           TEXT PRIMARY KEY,
    estimate_id  TEXT NOT NULL REFERENCES pm_estimate (id) ON DELETE CASCADE,
    key          INTEGER NOT NULL,
    value        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_pm_estimate_point_key ON pm_estimate_point (estimate_id, key);

-- A person deleted for good (people, merge, the engine's Delete forever) leaves no lead,
-- assignee or subscriber behind.
CREATE TRIGGER person_ad_pm_project AFTER DELETE ON person BEGIN
    UPDATE pm_project SET lead = NULL WHERE lead = old.id;
    UPDATE pm_project SET default_assignee = NULL WHERE default_assignee = old.id;
    UPDATE pm_project
       SET subscribers_json = (SELECT json_group_array(s.value) FROM json_each(pm_project.subscribers_json) AS s WHERE s.value <> old.id)
     WHERE EXISTS (SELECT 1 FROM json_each(pm_project.subscribers_json) AS s WHERE s.value = old.id);
END;
