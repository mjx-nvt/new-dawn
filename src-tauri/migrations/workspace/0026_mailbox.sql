-- 0.10.9 (D-066). Additive only: what Apple Mail answered for a record's addresses — threads,
-- messages and their participants — and the blocklist. Mail stays the source; these rows are
-- what the record's Emails widget shows between refreshes.

-- A conversation: the messages that share a subject (Re:, Fwd: and the like stripped).
CREATE TABLE message_thread (
    id                TEXT PRIMARY KEY,
    key               TEXT NOT NULL UNIQUE,
    subject           TEXT NOT NULL DEFAULT '',
    last_message_at   TEXT NOT NULL DEFAULT '',
    message_count     INTEGER NOT NULL DEFAULT 0,
    participants_json TEXT NOT NULL DEFAULT '[]',
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_message_thread_last ON message_thread (last_message_at);

-- One email as Mail gave it: its Message-ID, who sent it and when, which way it went, the
-- first words (per the visibility setting), the mailbox it sits in.
CREATE TABLE message (
    id             TEXT PRIMARY KEY,
    thread_id      TEXT NOT NULL REFERENCES message_thread (id) ON DELETE CASCADE,
    message_id     TEXT NOT NULL UNIQUE,
    subject        TEXT NOT NULL DEFAULT '',
    sender_handle  TEXT NOT NULL DEFAULT '',
    sender_name    TEXT NOT NULL DEFAULT '',
    received_at    TEXT NOT NULL DEFAULT '',
    direction      TEXT NOT NULL DEFAULT 'in' CHECK (direction IN ('in', 'out')),
    mailbox        TEXT NOT NULL DEFAULT '',
    snippet        TEXT NOT NULL DEFAULT '',
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_message_thread_received ON message (thread_id, received_at);

-- Who was on a message: the sender and every recipient, by address — the join that finds a
-- record's threads.
CREATE TABLE message_participant (
    id            TEXT PRIMARY KEY,
    message_id    TEXT NOT NULL REFERENCES message (id) ON DELETE CASCADE,
    role          TEXT NOT NULL DEFAULT 'to' CHECK (role IN ('from', 'to', 'cc', 'bcc')),
    handle        TEXT NOT NULL,
    display_name  TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_message_participant_handle ON message_participant (handle);
CREATE INDEX idx_message_participant_message ON message_participant (message_id);

-- Addresses and @domains never read from Mail nor proposed as people (Twenty's Blocklist).
CREATE TABLE blocklist (
    id          TEXT PRIMARY KEY,
    handle      TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
