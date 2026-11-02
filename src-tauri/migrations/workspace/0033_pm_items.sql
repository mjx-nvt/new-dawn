-- 0.11.1 (D-074): Plane's work items — each in a project, numbered there (VIFC-12: the number is
-- kept, the identifier composed when read, since identifiers can be renamed), with its state,
-- priority, assignees, labels, dates, estimate and parent; comments with one level of replies,
-- reactions, the activity that History and Transition read, the description's versions, the
-- people a comment mentions, and drafts. Enumerations are checked in Rust.
--   priority     urgent | high | medium | low | none
--   created_by, updated_by, actor
--                who did it: "me" (the owner) or, later, an agent or an automation
--   who, pm_reaction.created_by
--                "me" or a person's id (assignees, subscribers, mentions, reactions)
--   kind         created | updated | commented | archived | restored | copied (activity)

-- The number of the last item a project made: numbers are never given twice.
ALTER TABLE pm_project ADD COLUMN last_sequence INTEGER NOT NULL DEFAULT 0;

-- An item's state is one of its own project's (the composite key below needs this index).
CREATE UNIQUE INDEX idx_pm_state_project_id ON pm_state (project_id, id);

CREATE TABLE pm_item (
    id                 TEXT NOT NULL PRIMARY KEY,
    project_id         TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    sequence           INTEGER NOT NULL,
    name               TEXT NOT NULL,
    description_json   TEXT,
    description_text   TEXT NOT NULL DEFAULT '',
    state_id           TEXT NOT NULL,
    priority           TEXT NOT NULL DEFAULT 'none',
    start_date         TEXT,
    target_date        TEXT,
    estimate_point_id  TEXT REFERENCES pm_estimate_point (id) ON DELETE SET NULL,
    -- Deleting an item takes its sub-items (in Rust); the key only keeps no dangling parent.
    parent_id          TEXT REFERENCES pm_item (id) ON DELETE SET NULL,
    sort_order         REAL NOT NULL DEFAULT 65535,
    completed_at       TEXT,
    archived_at        TEXT,
    created_by         TEXT NOT NULL DEFAULT 'me',
    updated_by         TEXT NOT NULL DEFAULT 'me',
    created_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at         TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    -- No ON DELETE (NO ACTION, checked at the statement's end): a state items are in can't be
    -- deleted, while a project's delete still takes both; and a state of its own project only.
    FOREIGN KEY (project_id, state_id) REFERENCES pm_state (project_id, id),
    UNIQUE (project_id, sequence)
);

CREATE INDEX idx_pm_item_project ON pm_item (project_id, archived_at, sort_order);
CREATE INDEX idx_pm_item_parent ON pm_item (parent_id);
CREATE INDEX idx_pm_item_state ON pm_item (state_id);
CREATE INDEX idx_pm_item_estimate ON pm_item (estimate_point_id);

CREATE TABLE pm_item_assignee (
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    who         TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (item_id, who)
);

CREATE INDEX idx_pm_item_assignee_who ON pm_item_assignee (who);

CREATE TABLE pm_item_label (
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    label_id    TEXT NOT NULL REFERENCES pm_label (id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (item_id, label_id)
);

CREATE INDEX idx_pm_item_label_label ON pm_item_label (label_id);

CREATE TABLE pm_item_subscriber (
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    who         TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (item_id, who)
);

CREATE INDEX idx_pm_item_subscriber_who ON pm_item_subscriber (who);

-- Comments; a reply's parent is a comment that is not itself a reply (one level, in Rust).
CREATE TABLE pm_comment (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    parent_id   TEXT REFERENCES pm_comment (id) ON DELETE CASCADE,
    body_json   TEXT NOT NULL,
    body_text   TEXT NOT NULL DEFAULT '',
    created_by  TEXT NOT NULL DEFAULT 'me',
    edited_at   TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_comment_item ON pm_comment (item_id, created_at);
CREATE INDEX idx_pm_comment_parent ON pm_comment (parent_id);

-- Reactions on the description (no comment) or on a comment; one of each emoji a person.
CREATE TABLE pm_reaction (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    comment_id  TEXT REFERENCES pm_comment (id) ON DELETE CASCADE,
    emoji       TEXT NOT NULL,
    created_by  TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_pm_reaction_one ON pm_reaction (item_id, coalesce(comment_id, ''), emoji, created_by);
CREATE INDEX idx_pm_reaction_comment ON pm_reaction (comment_id);

-- What happened to an item, field by field (readable before and after).
CREATE TABLE pm_activity (
    id          TEXT NOT NULL PRIMARY KEY,
    item_id     TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    field       TEXT NOT NULL DEFAULT '',
    old_value   TEXT NOT NULL DEFAULT '',
    new_value   TEXT NOT NULL DEFAULT '',
    -- A state change keeps the states' ids too: Transition reads time in each state from them.
    old_ref     TEXT,
    new_ref     TEXT,
    actor       TEXT NOT NULL DEFAULT 'me',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_activity_item ON pm_activity (item_id, created_at);

CREATE TABLE pm_item_version (
    id                TEXT NOT NULL PRIMARY KEY,
    item_id           TEXT NOT NULL REFERENCES pm_item (id) ON DELETE CASCADE,
    description_json  TEXT,
    description_text  TEXT NOT NULL DEFAULT '',
    created_by        TEXT NOT NULL DEFAULT 'me',
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_item_version_item ON pm_item_version (item_id, created_at);

-- The people a comment mentions (the Inbox reads them, 0.11.2).
CREATE TABLE pm_mention (
    id          TEXT NOT NULL PRIMARY KEY,
    comment_id  TEXT NOT NULL REFERENCES pm_comment (id) ON DELETE CASCADE,
    who         TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_pm_mention_comment ON pm_mention (comment_id, who);
CREATE INDEX idx_pm_mention_who ON pm_mention (who);

-- Drafts: a work item not made yet — its fields as the create form holds them.
CREATE TABLE pm_draft (
    id          TEXT NOT NULL PRIMARY KEY,
    project_id  TEXT REFERENCES pm_project (project_id) ON DELETE SET NULL,
    data_json   TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_draft_project ON pm_draft (project_id);

-- A person deleted for good is no one's assignee, subscriber or mention any more.
CREATE TRIGGER person_ad_pm_item AFTER DELETE ON person BEGIN
    DELETE FROM pm_item_assignee WHERE who = old.id;
    DELETE FROM pm_item_subscriber WHERE who = old.id;
    DELETE FROM pm_mention WHERE who = old.id;
    DELETE FROM pm_reaction WHERE created_by = old.id;
END;
