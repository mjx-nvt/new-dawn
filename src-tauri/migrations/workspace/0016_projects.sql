-- 0.9.0 (D-054). Additive only: three new tables and a trigger; existing rows unchanged.

-- A project the owner follows in this workspace: what the Projects expert reads after each
-- summary, and what its proposals change once the owner applies them.
--   status          active | on_hold | done (checked in Rust)
--   milestone_due   YYYY-MM-DD, a local calendar day (NULL = no date)
--   risks_json, decisions_json   arrays of text, oldest first
CREATE TABLE project (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',
    phase           TEXT NOT NULL DEFAULT '',
    next_milestone  TEXT NOT NULL DEFAULT '',
    milestone_due   TEXT,
    risks_json      TEXT NOT NULL DEFAULT '[]',
    decisions_json  TEXT NOT NULL DEFAULT '[]',
    notes           TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Which meetings touched which project (found by the expert). The pair is the identity (INSERT
-- OR IGNORE relies on it), so no separate UUID.
CREATE TABLE project_meeting (
    project_id  TEXT NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    meeting_id  TEXT NOT NULL REFERENCES meeting (id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (project_id, meeting_id)
);

CREATE INDEX idx_project_meeting_meeting ON project_meeting (meeting_id);

-- A change the Projects expert proposed from a meeting, with its evidence (a sentence of the
-- summary); the owner applies or dismisses it, and the applied ones are the project's history.
-- Deleted with its project, and with its meeting (the evidence quotes the meeting).
--   field   milestone | phase | status | risk | decision | note (checked in Rust)
--   status  proposed | applied | dismissed (checked in Rust)
--   due     YYYY-MM-DD for a milestone
CREATE TABLE project_update (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    meeting_id  TEXT REFERENCES meeting (id) ON DELETE CASCADE,
    field       TEXT NOT NULL,
    value       TEXT NOT NULL,
    due         TEXT,
    evidence    TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'proposed',
    model       TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_project_update_project ON project_update (project_id, status, created_at);
CREATE INDEX idx_project_update_meeting ON project_update (meeting_id);

-- Before a meeting goes: decided updates (applied = the history, dismissed = not proposed again)
-- stay, unlinked and without the quote; the CASCADE then deletes the proposals still waiting.
-- BEFORE, because SQLite runs foreign-key actions ahead of AFTER triggers. updated_at is left
-- alone: the history is ordered by it.
CREATE TRIGGER meeting_bd_project_update BEFORE DELETE ON meeting BEGIN
    UPDATE project_update SET meeting_id = NULL, evidence = ''
    WHERE meeting_id = old.id AND status <> 'proposed';
END;
