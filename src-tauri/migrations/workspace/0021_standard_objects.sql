-- 0.10.1 (D-060). Additive only: the standard objects' tables and columns (Twenty's shape);
-- existing rows unchanged. Composite fields are side-by-side columns (<field>_<part>); a
-- many-to-one relation is <field>_id; every record table has position, created_by_* (the actor)
-- and deleted_at (a soft delete: Deleted records, restore, delete for good).

-- The one workspace member: the owner (the row is made at open from the profile).
CREATE TABLE workspace_member (
    id                 TEXT PRIMARY KEY,
    name_first         TEXT NOT NULL DEFAULT '',
    name_last          TEXT NOT NULL DEFAULT '',
    color_scheme       TEXT NOT NULL DEFAULT 'System',
    locale             TEXT NOT NULL DEFAULT 'en',
    avatar_url         TEXT NOT NULL DEFAULT '',
    user_email         TEXT NOT NULL DEFAULT '',
    time_zone          TEXT NOT NULL DEFAULT '',
    date_format        TEXT NOT NULL DEFAULT 'SYSTEM',
    time_format        TEXT NOT NULL DEFAULT 'SYSTEM',
    number_format      TEXT NOT NULL DEFAULT 'SYSTEM',
    calendar_start_day TEXT NOT NULL DEFAULT 'SYSTEM',
    position           REAL NOT NULL DEFAULT 0,
    deleted_at         TEXT,
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE company (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL DEFAULT '',
    domain_label           TEXT NOT NULL DEFAULT '',
    domain_url             TEXT NOT NULL DEFAULT '',
    domain_secondary       TEXT NOT NULL DEFAULT '[]',
    linkedin_label         TEXT NOT NULL DEFAULT '',
    linkedin_url           TEXT NOT NULL DEFAULT '',
    linkedin_secondary     TEXT NOT NULL DEFAULT '[]',
    x_label                TEXT NOT NULL DEFAULT '',
    x_url                  TEXT NOT NULL DEFAULT '',
    x_secondary            TEXT NOT NULL DEFAULT '[]',
    annual_revenue_micros  INTEGER,
    annual_revenue_code    TEXT NOT NULL DEFAULT '',
    address_street1        TEXT NOT NULL DEFAULT '',
    address_street2        TEXT NOT NULL DEFAULT '',
    address_city           TEXT NOT NULL DEFAULT '',
    address_state          TEXT NOT NULL DEFAULT '',
    address_country        TEXT NOT NULL DEFAULT '',
    address_postcode       TEXT NOT NULL DEFAULT '',
    address_lat            REAL,
    address_lng            REAL,
    employees              REAL,
    ideal_customer_profile INTEGER NOT NULL DEFAULT 0 CHECK (ideal_customer_profile IN (0, 1)),
    tagline                TEXT NOT NULL DEFAULT '',
    work_policy            TEXT NOT NULL DEFAULT '[]',
    visa_sponsorship       INTEGER NOT NULL DEFAULT 0 CHECK (visa_sponsorship IN (0, 1)),
    account_owner_id       TEXT REFERENCES workspace_member (id) ON DELETE SET NULL,
    position               REAL NOT NULL DEFAULT 0,
    created_by_source      TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name        TEXT NOT NULL DEFAULT '',
    created_by_member_id   TEXT,
    created_by_context     TEXT NOT NULL DEFAULT '{}',
    deleted_at             TEXT,
    created_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_company_name ON company (name COLLATE NOCASE);
CREATE INDEX idx_company_deleted ON company (deleted_at);
CREATE INDEX idx_company_owner ON company (account_owner_id);

CREATE TABLE opportunity (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL DEFAULT '',
    amount_micros        INTEGER,
    amount_code          TEXT NOT NULL DEFAULT '',
    close_date           TEXT,                        -- YYYY-MM-DD
    stage                TEXT NOT NULL DEFAULT 'NEW', -- NEW | SCREENING | MEETING | PROPOSAL | CUSTOMER (the field's options)
    probability          REAL,
    point_of_contact_id  TEXT REFERENCES person (id) ON DELETE SET NULL,
    company_id           TEXT REFERENCES company (id) ON DELETE SET NULL,
    owner_id             TEXT REFERENCES workspace_member (id) ON DELETE SET NULL,
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_opportunity_stage ON opportunity (stage, position);
CREATE INDEX idx_opportunity_company ON opportunity (company_id);
CREATE INDEX idx_opportunity_contact ON opportunity (point_of_contact_id);
CREATE INDEX idx_opportunity_close ON opportunity (close_date);
CREATE INDEX idx_opportunity_deleted ON opportunity (deleted_at);
CREATE INDEX idx_opportunity_owner ON opportunity (owner_id);

CREATE TABLE note (
    id                   TEXT PRIMARY KEY,
    title                TEXT NOT NULL DEFAULT '',
    body                 TEXT NOT NULL DEFAULT '',   -- plain text; the editor's JSON from 0.10.6
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_note_deleted ON note (deleted_at);

-- A note attached to a record (one row per record; a note can have several). Goes with the note,
-- and with the record it points at.
CREATE TABLE note_target (
    id             TEXT PRIMARY KEY,
    note_id        TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    person_id      TEXT REFERENCES person (id) ON DELETE CASCADE,
    company_id     TEXT REFERENCES company (id) ON DELETE CASCADE,
    opportunity_id TEXT REFERENCES opportunity (id) ON DELETE CASCADE,
    position       REAL NOT NULL DEFAULT 0,
    deleted_at     TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_note_target_note ON note_target (note_id);
CREATE INDEX idx_note_target_person ON note_target (person_id);
CREATE INDEX idx_note_target_company ON note_target (company_id);
CREATE INDEX idx_note_target_opportunity ON note_target (opportunity_id);

-- A task attached to a record, as a note is.
CREATE TABLE task_target (
    id             TEXT PRIMARY KEY,
    task_id        TEXT NOT NULL REFERENCES task (id) ON DELETE CASCADE,
    person_id      TEXT REFERENCES person (id) ON DELETE CASCADE,
    company_id     TEXT REFERENCES company (id) ON DELETE CASCADE,
    opportunity_id TEXT REFERENCES opportunity (id) ON DELETE CASCADE,
    position       REAL NOT NULL DEFAULT 0,
    deleted_at     TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_task_target_task ON task_target (task_id);
CREATE INDEX idx_task_target_person ON task_target (person_id);
CREATE INDEX idx_task_target_company ON task_target (company_id);
CREATE INDEX idx_task_target_opportunity ON task_target (opportunity_id);

-- A file on a record (the Files widget, 0.10.6). The file lives under the workspace folder.
CREATE TABLE attachment (
    id                   TEXT PRIMARY KEY,
    name                 TEXT NOT NULL DEFAULT '',
    full_path            TEXT NOT NULL DEFAULT '',
    type                 TEXT NOT NULL DEFAULT '',
    author_id            TEXT REFERENCES workspace_member (id) ON DELETE SET NULL,
    person_id            TEXT REFERENCES person (id) ON DELETE CASCADE,
    company_id           TEXT REFERENCES company (id) ON DELETE CASCADE,
    opportunity_id       TEXT REFERENCES opportunity (id) ON DELETE CASCADE,
    note_id              TEXT REFERENCES note (id) ON DELETE CASCADE,
    task_id              TEXT REFERENCES task (id) ON DELETE CASCADE,
    position             REAL NOT NULL DEFAULT 0,
    created_by_source    TEXT NOT NULL DEFAULT 'MANUAL',
    created_by_name      TEXT NOT NULL DEFAULT '',
    created_by_member_id TEXT,
    created_by_context   TEXT NOT NULL DEFAULT '{}',
    deleted_at           TEXT,
    created_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at           TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_attachment_person ON attachment (person_id);
CREATE INDEX idx_attachment_company ON attachment (company_id);
CREATE INDEX idx_attachment_opportunity ON attachment (opportunity_id);
CREATE INDEX idx_attachment_note ON attachment (note_id);
CREATE INDEX idx_attachment_task ON attachment (task_id);
CREATE INDEX idx_attachment_author ON attachment (author_id);

-- A favorite in the sidebar (0.10.7): a record, later a view, in a folder.
CREATE TABLE favorite (
    id                  TEXT PRIMARY KEY,
    position            REAL NOT NULL DEFAULT 0,
    workspace_member_id TEXT REFERENCES workspace_member (id) ON DELETE SET NULL,
    person_id           TEXT REFERENCES person (id) ON DELETE CASCADE,
    company_id          TEXT REFERENCES company (id) ON DELETE CASCADE,
    opportunity_id      TEXT REFERENCES opportunity (id) ON DELETE CASCADE,
    note_id             TEXT REFERENCES note (id) ON DELETE CASCADE,
    task_id             TEXT REFERENCES task (id) ON DELETE CASCADE,
    deleted_at          TEXT,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_favorite_position ON favorite (position);
CREATE INDEX idx_favorite_member ON favorite (workspace_member_id);
CREATE INDEX idx_favorite_person ON favorite (person_id);
CREATE INDEX idx_favorite_company ON favorite (company_id);
CREATE INDEX idx_favorite_opportunity ON favorite (opportunity_id);
CREATE INDEX idx_favorite_note ON favorite (note_id);
CREATE INDEX idx_favorite_task ON favorite (task_id);

-- People (the memory, D-028) in Twenty's shape. `name`, `title`, `organisation`, `aliases_json`
-- and `notes` stay: the meeting pipeline reads them. The engine keeps `name` = first + last and
-- `organisation` = the company's name; the memory's editor keeps the new columns.
ALTER TABLE person ADD COLUMN name_first TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN name_last TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN emails_primary TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN emails_additional TEXT NOT NULL DEFAULT '[]';
ALTER TABLE person ADD COLUMN phones_number TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN phones_country TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN phones_calling TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN phones_additional TEXT NOT NULL DEFAULT '[]';
ALTER TABLE person ADD COLUMN linkedin_label TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN linkedin_url TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN linkedin_secondary TEXT NOT NULL DEFAULT '[]';
ALTER TABLE person ADD COLUMN x_label TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN x_url TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN x_secondary TEXT NOT NULL DEFAULT '[]';
ALTER TABLE person ADD COLUMN city TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN avatar_url TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN company_id TEXT REFERENCES company (id) ON DELETE SET NULL;
ALTER TABLE person ADD COLUMN position REAL NOT NULL DEFAULT 0;
ALTER TABLE person ADD COLUMN created_by_source TEXT NOT NULL DEFAULT 'MANUAL';
ALTER TABLE person ADD COLUMN created_by_name TEXT NOT NULL DEFAULT '';
ALTER TABLE person ADD COLUMN created_by_member_id TEXT;
ALTER TABLE person ADD COLUMN created_by_context TEXT NOT NULL DEFAULT '{}';
ALTER TABLE person ADD COLUMN deleted_at TEXT;
CREATE INDEX idx_person_company ON person (company_id);
CREATE INDEX idx_person_deleted ON person (deleted_at);
CREATE INDEX idx_person_email ON person (emails_primary);
-- The people memory deletes persons directly (remove, merge): keep the engine's index and
-- timeline in step (they have no foreign keys, by design).
CREATE TRIGGER person_ad_engine AFTER DELETE ON person BEGIN
    DELETE FROM search_index WHERE object = 'person' AND record_id = old.id;
    DELETE FROM timeline_activity WHERE person_id = old.id;
END;

-- Tasks (D-042) in Twenty's shape. `status` mirrors `done_at` (the writers keep them in step).
ALTER TABLE task ADD COLUMN body TEXT NOT NULL DEFAULT '';
ALTER TABLE task ADD COLUMN status TEXT NOT NULL DEFAULT 'TODO';
ALTER TABLE task ADD COLUMN assignee_id TEXT REFERENCES workspace_member (id) ON DELETE SET NULL;
ALTER TABLE task ADD COLUMN position REAL NOT NULL DEFAULT 0;
ALTER TABLE task ADD COLUMN created_by_source TEXT NOT NULL DEFAULT 'MANUAL';
ALTER TABLE task ADD COLUMN created_by_name TEXT NOT NULL DEFAULT '';
ALTER TABLE task ADD COLUMN created_by_member_id TEXT;
ALTER TABLE task ADD COLUMN created_by_context TEXT NOT NULL DEFAULT '{}';
UPDATE task SET status = 'DONE' WHERE done_at IS NOT NULL;
CREATE INDEX idx_task_status ON task (status, deleted_at);
CREATE INDEX idx_task_assignee ON task (assignee_id);
-- status mirrors done_at for every writer (the Tasks screen, the meeting pipeline, the engine);
-- IN_PROGRESS is left alone. recursive_triggers is off: the inner UPDATE can't loop.
CREATE TRIGGER task_ai_status AFTER INSERT ON task
WHEN new.done_at IS NOT NULL AND new.status <> 'DONE' BEGIN
    UPDATE task SET status = 'DONE' WHERE id = new.id;
END;
CREATE TRIGGER task_au_status AFTER UPDATE OF done_at ON task
WHEN (new.done_at IS NOT NULL) <> (new.status = 'DONE') BEGIN
    UPDATE task SET status = CASE WHEN new.done_at IS NULL THEN 'TODO' ELSE 'DONE' END WHERE id = new.id;
END;
