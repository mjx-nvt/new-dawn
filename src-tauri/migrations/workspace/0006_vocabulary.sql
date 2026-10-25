-- Vocabulary (D-036): the workspace's terms and the ways Whisper mishears them. Guides Whisper
-- (initial prompt) and corrects transcripts right after transcription. `source` is 'learned'
-- (from the owner's transcript corrections) or 'manual'. Additive only.

CREATE TABLE vocabulary (
    id            TEXT PRIMARY KEY,                -- UUID v7
    term          TEXT NOT NULL,                   -- the right spelling
    heard_as_json TEXT NOT NULL DEFAULT '[]',      -- mishearings, JSON array of strings
    source        TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'learned')),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE UNIQUE INDEX idx_vocabulary_term ON vocabulary (term COLLATE NOCASE);

-- What Whisper wrote for a line the vocabulary corrected (NULL = not auto-corrected), so the
-- correction can be undone.
ALTER TABLE transcript_segment ADD COLUMN raw_text TEXT;
