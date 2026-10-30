-- 0.8.1 (D-049). Additive only: a new table and a new nullable column; existing rows unchanged.

-- The feedback ledger: every decision the owner makes about what the AI wrote — a transcript
-- line fixed, a correction, a section edited, a name confirmed or kept, an attendee changed, an
-- action item sent to Tasks, a task the AI added reworded or deleted, an email written again.
-- Append-only, text only (never audio). Deleted with its meeting (rows with no meeting stay).
--   agent   which AI wrote what was judged: transcript | summary | naming | email | tasks (checked in Rust)
--   kind    the decision (checked in Rust)
--   before_text / after_text   the AI's text and the owner's ('' when there's none)
--   context_json               the section, the model, the reason… ('{}' when none)
CREATE TABLE feedback (
    id           TEXT PRIMARY KEY,
    meeting_id   TEXT REFERENCES meeting (id) ON DELETE CASCADE,
    agent        TEXT NOT NULL,
    kind         TEXT NOT NULL,
    before_text  TEXT NOT NULL DEFAULT '',
    after_text   TEXT NOT NULL DEFAULT '',
    context_json TEXT NOT NULL DEFAULT '{}',
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_feedback_agent ON feedback (agent, kind, created_at);
CREATE INDEX idx_feedback_meeting ON feedback (meeting_id);

-- The summary as the AI wrote it (the same JSON as sections_json at that moment), kept beside the
-- owner's edits: what the ledger and the examples compare with. NULL for summaries written before.
ALTER TABLE summary ADD COLUMN ai_sections_json TEXT;
