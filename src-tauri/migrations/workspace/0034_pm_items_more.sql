-- 0.11.2 (D-075): Plane's work items, the rest — relations between items, links, attachments,
-- the owner's vote — and the Inbox's notifications. Enumerations are checked in Rust.
--   pm_relation.kind    relates_to | duplicate | blocked_by ("Blocking" is blocked_by read from
--                       the other item)
--   notification.kind   property | state | completed | comment | mention | reminder
--   created_by          who did it: "me" (the owner), an agent or "automation"
-- The Inbox's preferences are an app setting (one owner), not a table; the automations' last day
-- is a workspace setting.

CREATE TABLE pm_relation (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    related_id  TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    created_by  TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (item_id <> related_id)
);

-- One relation a pair of items, whichever side it was made from.
CREATE UNIQUE INDEX idx_pm_relation_pair ON pm_relation (min(item_id, related_id), max(item_id, related_id));
CREATE INDEX idx_pm_relation_item ON pm_relation (item_id);
CREATE INDEX idx_pm_relation_related ON pm_relation (related_id);

CREATE TABLE pm_link (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    url         TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_link_item ON pm_link (item_id, created_at);
-- An address once an item (Plane refuses the same link twice).
CREATE UNIQUE INDEX idx_pm_link_url ON pm_link (item_id, url);

-- A file attached to an item, kept in the workspace's files/pm/<item id>/ under `file`.
CREATE TABLE pm_attachment (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    -- A name inside that folder, never a path: an absolute one or '..' would lead out of it.
    file        TEXT NOT NULL CHECK (file <> '' AND instr(file, '/') = 0 AND file NOT IN ('.', '..')),
    size        INTEGER NOT NULL DEFAULT 0,
    created_by  TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_attachment_item ON pm_attachment (item_id, created_at);

-- Upvote (1) or downvote (-1); the owner's only — people don't vote.
CREATE TABLE pm_vote (
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    who         TEXT NOT NULL DEFAULT 'me',
    vote        INTEGER NOT NULL CHECK (vote IN (-1, 1)),
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (item_id, who)
);

-- The owner's Inbox: what someone else (an agent, an automation) did on an item the owner
-- follows — the activity row it reads —, a mention, or a due-date reminder (due_on, once a due
-- date).
CREATE TABLE notification (
    id             TEXT NOT NULL PRIMARY KEY,
    kind           TEXT NOT NULL,
    item_id        TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    activity_id    TEXT REFERENCES pm_activity (id) ON DELETE CASCADE,
    actor          TEXT NOT NULL DEFAULT '',
    due_on         TEXT,
    read_at        TEXT,
    snoozed_until  TEXT,
    archived_at    TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- A reminder is of a due date (its uniqueness below can't hold a NULL).
    CHECK (kind <> 'reminder' OR due_on IS NOT NULL)
);

CREATE INDEX idx_notification_inbox ON notification (archived_at, created_at);
CREATE INDEX idx_notification_item ON notification (item_id);
CREATE INDEX idx_notification_activity ON notification (activity_id);
CREATE UNIQUE INDEX idx_notification_reminder ON notification (item_id, due_on) WHERE kind = 'reminder';
