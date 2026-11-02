-- 0.11.8 (D-081): Plane's intake — requests waiting to become work. An intake item is a work item
-- in its project's Triage state (made when Intake is first used; its items show only in Intake),
-- with how it stands here: pending, accepted (it moved to a state of the project's work),
-- declined, snoozed until a day (it reads pending again from that day), or a duplicate of another
-- item. Made in the app or filed from an email by the Email reading agent.
--   status   pending | accepted | declined | snoozed | duplicate (checked in Rust)
--   source   in_app | email

CREATE TABLE pm_intake_item (
    item_id        TEXT NOT NULL PRIMARY KEY REFERENCES pm_item (id) ON DELETE CASCADE,
    status         TEXT NOT NULL DEFAULT 'pending',
    -- A day (YYYY-MM-DD) while snoozed.
    snoozed_until  TEXT,
    -- The item it repeats, while a duplicate.
    duplicate_of   TEXT REFERENCES pm_item (id) ON DELETE SET NULL,
    source         TEXT NOT NULL DEFAULT 'in_app',
    -- The email it came from (its reading is kept, D-056).
    email_item_id  TEXT REFERENCES email_item (id) ON DELETE SET NULL,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK (date(snoozed_until) IS snoozed_until),
    -- A snooze has its day (without one it never reads pending again); nothing duplicates itself.
    CHECK (status <> 'snoozed' OR snoozed_until IS NOT NULL),
    CHECK (duplicate_of IS NOT item_id)
);

CREATE INDEX idx_pm_intake_item_status ON pm_intake_item (status);
CREATE INDEX idx_pm_intake_item_duplicate ON pm_intake_item (duplicate_of);
CREATE INDEX idx_pm_intake_item_email ON pm_intake_item (email_item_id);
