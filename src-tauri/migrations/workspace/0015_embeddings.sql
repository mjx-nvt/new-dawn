-- 0.8.5 (D-053). Additive only: two new tables; existing rows unchanged.

-- Search by meaning: each meeting's transcript (in pieces of a few lines) and summary (by
-- section) as vectors from the embedding model (bge-m3 through Ollama), computed in the
-- background. `text` keeps the piece for the result's snippet; `text_hash` tells a piece that
-- changed (a transcript run again, a section edited) from one that didn't. Deleted with the meeting.
--   source  'transcript' | 'summary' (checked in Rust)
--   start_ms  where a transcript piece starts (NULL for the summary)
--   vector  little-endian f32s (1024 for bge-m3); `model` names the model that made them, so
--           another model's vectors are never compared with a query's
CREATE TABLE embedding (
    id          TEXT PRIMARY KEY,
    meeting_id  TEXT NOT NULL REFERENCES meeting (id) ON DELETE CASCADE,
    source      TEXT NOT NULL,
    start_ms    INTEGER,
    text        TEXT NOT NULL,
    text_hash   TEXT NOT NULL,
    model       TEXT NOT NULL,
    vector      BLOB NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Serves the cascade and a meeting's rows; search reads every row anyway.
CREATE INDEX idx_embedding_meeting ON embedding (meeting_id, model);

-- When each meeting was last indexed, with which model — one row per meeting, so a meeting with
-- nothing to embed (a silent recording) counts as done, and checking a meeting never rewrites
-- its vectors. The sweep re-indexes a meeting whose transcript or summary changed after this.
CREATE TABLE embedding_check (
    meeting_id  TEXT PRIMARY KEY REFERENCES meeting (id) ON DELETE CASCADE,
    model       TEXT NOT NULL,
    checked_at  TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
