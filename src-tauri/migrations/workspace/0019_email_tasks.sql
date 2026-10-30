-- 0.9.2 (D-056). Additive only: a new table; existing rows unchanged.

-- A task the Email reading agent proposed for the owner from an email. It waits under the email
-- until the owner adds it to Tasks or dismisses it: experts propose, the owner applies (D-054).
-- Forgetting the email takes its proposals; a task already added stays in Tasks.
--   item       the task as proposed
--   due_text   the date as the email says it ('' = none)
--   evidence   the sentence of the email it rests on
--   status     proposed | added | dismissed (checked in Rust)
--   task_id    the task made from it, once added
CREATE TABLE email_task (
    id            TEXT PRIMARY KEY,
    email_item_id TEXT NOT NULL REFERENCES email_item (id) ON DELETE CASCADE,
    item          TEXT NOT NULL,
    due_text      TEXT NOT NULL DEFAULT '',
    evidence      TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'proposed',
    task_id       TEXT REFERENCES task (id) ON DELETE SET NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_email_task_email ON email_task (email_item_id, created_at);
CREATE INDEX idx_email_task_task ON email_task (task_id);
