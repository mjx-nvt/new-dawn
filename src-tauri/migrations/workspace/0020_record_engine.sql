-- 0.10.1 (D-060). Additive only: the record engine's metadata, its timeline and its search index.
-- No existing table changes.

-- An object of the CRM — Twenty's data model: a real table described here. The standard objects
-- are declared in Rust (kernel/records/standard.rs) and synced into this table at every open;
-- custom ones are made at run time (0.10.5). Rows are never deleted by the app: deactivated.
--   name_singular / name_plural   camelCase names (person / people), unique
--   table_name                    the table that holds the records
--   is_system                     hidden from the sidebar and the data model (targets, favorites…)
--   label_identifier_field_id     the field whose value names a record; image_… its picture
CREATE TABLE object_meta (
    id                        TEXT PRIMARY KEY,
    name_singular             TEXT NOT NULL UNIQUE,
    name_plural               TEXT NOT NULL UNIQUE,
    label_singular            TEXT NOT NULL,
    label_plural              TEXT NOT NULL,
    icon                      TEXT NOT NULL DEFAULT '',
    description               TEXT NOT NULL DEFAULT '',
    table_name                TEXT NOT NULL UNIQUE,
    is_custom                 INTEGER NOT NULL DEFAULT 0 CHECK (is_custom IN (0, 1)),
    is_active                 INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    is_system                 INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    label_identifier_field_id TEXT,
    image_identifier_field_id TEXT,
    position                  REAL NOT NULL DEFAULT 0,
    created_at                TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at                TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- A field of an object. `type` is Twenty's field type (TEXT, NUMBER, SELECT, FULL_NAME, RELATION…,
-- checked in Rust); `column_name` the base column — a composite field occupies <column>_<part>
-- columns, a many-to-one relation <column>_id, a one-to-many relation none. `options_json` holds
-- select options (value, label, colour, position), number / currency / phone defaults and the
-- relation's target, kind and inverse field.
CREATE TABLE field_meta (
    id            TEXT PRIMARY KEY,
    object_id     TEXT NOT NULL REFERENCES object_meta (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    label         TEXT NOT NULL,
    type          TEXT NOT NULL,
    column_name   TEXT NOT NULL DEFAULT '',
    options_json  TEXT NOT NULL DEFAULT '{}',
    default_json  TEXT,
    is_custom     INTEGER NOT NULL DEFAULT 0 CHECK (is_custom IN (0, 1)),
    is_active     INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    is_nullable   INTEGER NOT NULL DEFAULT 1 CHECK (is_nullable IN (0, 1)),
    is_unique     INTEGER NOT NULL DEFAULT 0 CHECK (is_unique IN (0, 1)),
    is_system     INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    position      REAL NOT NULL DEFAULT 0,
    description   TEXT NOT NULL DEFAULT '',
    icon          TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (object_id, name)
);

-- What happened to a record: <object>.created / .updated (with the diff in properties_json) /
-- .deleted / .restored, and later the notes, tasks, emails and meetings linked to it. One target
-- column per record-bearing object; a custom object adds its own column when it is made.
-- No foreign keys on the targets on purpose: the engine deletes a record's activities itself.
CREATE TABLE timeline_activity (
    id                        TEXT PRIMARY KEY,
    happens_at                TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    name                      TEXT NOT NULL,
    properties_json           TEXT NOT NULL DEFAULT '{}',
    workspace_member_id       TEXT,
    linked_record_cached_name TEXT NOT NULL DEFAULT '',
    linked_record_id          TEXT,
    linked_object_metadata_id TEXT,
    person_id                 TEXT,
    company_id                TEXT,
    opportunity_id            TEXT,
    note_id                   TEXT,
    task_id                   TEXT,
    created_at                TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at                TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_timeline_person ON timeline_activity (person_id, happens_at);
CREATE INDEX idx_timeline_company ON timeline_activity (company_id, happens_at);
CREATE INDEX idx_timeline_opportunity ON timeline_activity (opportunity_id, happens_at);
CREATE INDEX idx_timeline_note ON timeline_activity (note_id, happens_at);
CREATE INDEX idx_timeline_task ON timeline_activity (task_id, happens_at);

-- The search index over every object's text, kept by the engine on each write (no triggers:
-- custom tables come and go). `object` is the singular name; `text` the record's words.
CREATE VIRTUAL TABLE search_index USING fts5 (
    object UNINDEXED,
    record_id UNINDEXED,
    text,
    tokenize = 'unicode61 remove_diacritics 2'
);
