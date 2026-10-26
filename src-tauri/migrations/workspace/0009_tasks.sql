-- Tasks (0.7.3): the owner's to-dos in this workspace — typed in Tasks, added from a meeting's
-- action items ("Add to Tasks"), or found by the tasks agent after a summary. A linked action item
-- carries the task's id in the summary JSON (`task_id`), and the tick follows both ways.
-- `deleted_at` is a tombstone: the task is hidden and never made again automatically; "Add to
-- Tasks" on its item brings it back. `source_key` makes adding an item safe to repeat (NULLs are
-- distinct, so typed tasks and tasks whose meeting was deleted never collide). `meeting.tasks_at`:
-- when the meeting first fed Tasks — automatic tasks are made only before that. Additive only.

CREATE TABLE task (
    id          TEXT PRIMARY KEY,                       -- UUID v7
    title       TEXT NOT NULL,
    owner       TEXT NOT NULL DEFAULT '',               -- the action item's owner when it isn't the user ("Minh")
    due_text    TEXT NOT NULL DEFAULT '',               -- the phrase as said ('' = no date)
    due_on      TEXT,                                   -- YYYY-MM-DD, a local calendar day
    done_at     TEXT,
    meeting_id  TEXT REFERENCES meeting (id) ON DELETE SET NULL,
    source_key  TEXT,                                   -- fold(owner)|fold(item) at creation; NULL for typed tasks
    origin      TEXT NOT NULL CHECK (origin IN ('auto', 'button', 'typed')),
    reason      TEXT NOT NULL DEFAULT '',               -- why the AI thought it's yours (auto only)
    deleted_at  TEXT,                                   -- tombstone: hidden, never recreated automatically
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_task_source ON task (meeting_id, source_key);
CREATE INDEX idx_task_open ON task (deleted_at, done_at, due_on);

ALTER TABLE meeting ADD COLUMN tasks_at TEXT;           -- when this meeting first fed the task list
