-- 0.9.1 (D-055). Additive only: a new column, a new table and a trigger; existing rows unchanged.

-- What the owner keeps about a person (their dossier's notes).
ALTER TABLE person ADD COLUMN notes TEXT NOT NULL DEFAULT '';

-- What someone said they would do: a meeting's action item whose owner isn't the meeting owner
-- (the owner's own go to Tasks). Proposed from each new summary; the owner confirms it (open),
-- then marks it kept or dropped, or dismisses it (not a commitment).
--   person_id    the person it names, when the people memory knows them (else only the name)
--   person_name  as the summary named them (a person or an organisation)
--   due_text     the date as said ('' = none)
--   source_key   fold(owner)|fold(item): one commitment per action item per meeting
--   status       proposed | open | kept | dropped | dismissed (checked in Rust)
CREATE TABLE commitment (
    id          TEXT PRIMARY KEY,
    person_id   TEXT REFERENCES person (id) ON DELETE SET NULL,
    person_name TEXT NOT NULL,
    what        TEXT NOT NULL,
    due_text    TEXT NOT NULL DEFAULT '',
    meeting_id  TEXT REFERENCES meeting (id) ON DELETE SET NULL,
    source_key  TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'proposed',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_commitment_source ON commitment (meeting_id, source_key);
CREATE INDEX idx_commitment_person ON commitment (person_id, status);
CREATE INDEX idx_commitment_status ON commitment (status, created_at);

-- Before a meeting goes: its proposals still waiting go with it; decided commitments stay as the
-- owner's record (as tasks do, D-042), unlinked by the SET NULL. BEFORE, because SQLite runs
-- foreign-key actions ahead of AFTER triggers (meeting_id would already be NULL).
CREATE TRIGGER meeting_bd_commitment BEFORE DELETE ON meeting BEGIN
    DELETE FROM commitment WHERE meeting_id = old.id AND status = 'proposed';
END;
