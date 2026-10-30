-- 0.8.4 (D-052). Additive only: a new table and a trigger; existing rows unchanged.

-- "What I learned": lines the app proposes for an agent's guide after reading the owner's
-- decisions in the feedback ledger, each with its evidence. Never applied by themselves: Approve
-- appends the line to the guide, Dismiss keeps it only so it isn't proposed again.
--   agent        the agent whose guide it is for: summary | tasks (checked in Rust)
--   status       proposed | approved | dismissed (checked in Rust)
--   evidence     "You shortened the overview in 4 of 5 meetings"
--   feedback_ids_json  the ledger rows it rests on (they may be deleted with their meeting later)
--   model        the text model that proposed it
CREATE TABLE lesson (
    id                TEXT PRIMARY KEY,
    agent             TEXT NOT NULL,
    line              TEXT NOT NULL,
    evidence          TEXT NOT NULL DEFAULT '',
    feedback_ids_json TEXT NOT NULL DEFAULT '[]',
    status            TEXT NOT NULL DEFAULT 'proposed',
    model             TEXT,
    created_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_lesson_agent ON lesson (agent, status, created_at);

-- A lesson is the model's reading of ledger rows, which hold a meeting's words: when one goes with
-- its meeting (0012), a proposal resting on it goes too (it can be proposed again from what's
-- left), and a decided lesson keeps only its line (still needed so it isn't proposed again).
CREATE TRIGGER feedback_ad_lesson AFTER DELETE ON feedback BEGIN
    DELETE FROM lesson WHERE status = 'proposed' AND instr(feedback_ids_json, old.id) > 0;
    UPDATE lesson SET evidence = '', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    WHERE evidence <> '' AND instr(feedback_ids_json, old.id) > 0;
END;
