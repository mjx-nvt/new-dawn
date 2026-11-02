-- 0.10.15 (D-072): the CRM agent's proposals — the people, companies, opportunities, notes and
-- tasks it read in a meeting's summary or in an email, each with the sentence that says so,
-- waiting for the owner to apply (written through the record engine) or dismiss (D-054).
--   kind        company | person | opportunity | note | task
--   name        the record's name (a note's or a task's title)
--   about_*     the record it belongs to — a person's or an opportunity's company, the record a
--               note or a task is attached to: its object, its name as said, and its id once it
--               exists (a company proposed beside it has none until it is applied)
--   data_json   the fields to write, in the engine's names (relations are set at Apply)
--   status      proposed | applied | dismissed
--   record_id   what Apply wrote (a task already in Tasks is attached, not made again)
-- A proposal goes with its meeting or its email; what was applied stays in the CRM.
CREATE TABLE crm_proposal (
    id            TEXT PRIMARY KEY,
    meeting_id    TEXT REFERENCES meeting (id) ON DELETE CASCADE,
    email_item_id TEXT REFERENCES email_item (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL,                     -- company | person | opportunity | note | task (checked in Rust)
    name          TEXT NOT NULL,
    about_object  TEXT,
    about_name    TEXT NOT NULL DEFAULT '',
    about_id      TEXT,
    data_json     TEXT NOT NULL DEFAULT '{}',
    quote         TEXT NOT NULL,
    model         TEXT,
    status        TEXT NOT NULL DEFAULT 'proposed',  -- proposed | applied | dismissed (checked in Rust)
    record_id     TEXT,
    decided_at    TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- Exactly one source. Both keys CASCADE: SET NULL would fail this CHECK and abort the
    -- meeting's or the email's delete.
    CHECK ((meeting_id IS NULL) <> (email_item_id IS NULL))
);

CREATE INDEX idx_crm_proposal_status ON crm_proposal (status, created_at);
CREATE INDEX idx_crm_proposal_meeting ON crm_proposal (meeting_id);
CREATE INDEX idx_crm_proposal_email ON crm_proposal (email_item_id);
CREATE INDEX idx_crm_proposal_about ON crm_proposal (about_id) WHERE about_id IS NOT NULL;
