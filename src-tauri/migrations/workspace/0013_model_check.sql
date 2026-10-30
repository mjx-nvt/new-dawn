-- 0.8.2 (D-050). Additive only: a new column with a default, an index, a new table and a trigger;
-- existing rows unchanged.

-- Meetings the owner chose as references: their corrected transcript and edited summary are what
-- a candidate model is compared with (Settings → Models → Try a model on your meetings).
ALTER TABLE meeting ADD COLUMN reference INTEGER NOT NULL DEFAULT 0 CHECK (reference IN (0, 1));

-- Only reference meetings are in it; every model-check query filters on them.
CREATE INDEX idx_meeting_reference ON meeting (held_on) WHERE reference = 1;

-- One run of the model check: a candidate model against the one in use, on the reference
-- meetings, in the background. `result_json` holds the numbers per meeting and the blind pairs
-- (both texts and which model wrote which); `picks_json` the owner's choices on those pairs
-- (pair id → 'a' | 'b' | 'same').
--   kind    'speech' | 'text' (checked in Rust: the speaker models, 0.8.4–0.8.5, add one)
--   status  'waiting' | 'running' | 'done' | 'failed' | 'cancelled' (checked in Rust)
CREATE TABLE model_check (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    baseline    TEXT NOT NULL,
    candidate   TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'waiting',
    progress    INTEGER NOT NULL DEFAULT 0,
    result_json TEXT,
    picks_json  TEXT NOT NULL DEFAULT '{}',
    error       TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_model_check_created ON model_check (created_at);

-- A check holds copies of its meetings' words (titles, transcript windows, summary sections), so
-- deleting a meeting deletes the checks that hold them, as its feedback goes with it (0012). Every
-- pair id in result_json starts with its meeting's id (model_check.rs `pair`). A check still
-- waiting or running when a reference meeting is deleted goes too: it would write that meeting's
-- words when it finishes.
CREATE TRIGGER meeting_ad_model_check AFTER DELETE ON meeting BEGIN
    DELETE FROM model_check
    WHERE instr(result_json, old.id) > 0
       OR (old.reference = 1 AND status IN ('waiting', 'running'));
END;
