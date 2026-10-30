-- 0.9.2 (D-056). Additive only: a new table, two columns and a trigger; existing rows unchanged.

-- An email the owner gave New Dawn to read (the one selected in Apple Mail, or an .eml file),
-- and what came of it. Kept so it's read once and its proposals can quote it; the owner can
-- forget it.
--   message_id  the Message-ID header ('' when the mail has none): an email is read once
--   source      mail | eml (checked in Rust)
--   tasks, updates, commitments   how many of each the Email reading agent proposed or added
CREATE TABLE email_item (
    id            TEXT PRIMARY KEY,
    message_id    TEXT NOT NULL DEFAULT '',
    subject       TEXT NOT NULL DEFAULT '',
    sender        TEXT NOT NULL DEFAULT '',
    received_text TEXT NOT NULL DEFAULT '',  -- the date as the mail says it
    source        TEXT NOT NULL,
    tasks         INTEGER NOT NULL DEFAULT 0,
    updates       INTEGER NOT NULL DEFAULT 0,
    commitments   INTEGER NOT NULL DEFAULT 0,
    model         TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    body          TEXT NOT NULL DEFAULT ''   -- last: listing never reads its overflow pages
);

CREATE UNIQUE INDEX idx_email_item_message ON email_item (message_id) WHERE message_id <> '';
CREATE INDEX idx_email_item_created ON email_item (created_at);

-- Proposals that came from an email instead of a meeting.
ALTER TABLE project_update ADD COLUMN email_item_id TEXT REFERENCES email_item (id) ON DELETE SET NULL;
ALTER TABLE commitment ADD COLUMN email_item_id TEXT REFERENCES email_item (id) ON DELETE SET NULL;

CREATE INDEX idx_project_update_email ON project_update (email_item_id);
-- One commitment per email and item (as idx_commitment_source does per meeting).
CREATE UNIQUE INDEX idx_commitment_email ON commitment (email_item_id, source_key);

-- Before an email is forgotten: its proposals still waiting go with it; decided ones stay
-- (unlinked by the SET NULL), and a project update loses its quote of the email.
CREATE TRIGGER email_item_bd BEFORE DELETE ON email_item BEGIN
    DELETE FROM project_update WHERE email_item_id = old.id AND status = 'proposed';
    UPDATE project_update SET evidence = '' WHERE email_item_id = old.id;
    DELETE FROM commitment WHERE email_item_id = old.id AND status = 'proposed';
END;
