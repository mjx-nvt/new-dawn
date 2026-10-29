-- 0.7.7 (D-046). Additive only: a new table; existing rows unchanged.

-- The summary in another language, translated on request: one row per meeting and language
-- ('vi' | 'fr' | 'de' | 'ja' | 'ko' | 'zh' | 'es', checked in Rust like meeting.language).
-- `source_key` is a fingerprint (SHA-256) of the summary's words it was made from; a different one
-- (a re-run, an edit, a rename) makes the translation out of date.
CREATE TABLE summary_translation (
    id                  TEXT PRIMARY KEY,
    meeting_id          TEXT NOT NULL REFERENCES meeting (id) ON DELETE CASCADE,
    lang                TEXT NOT NULL,
    doc_json            TEXT NOT NULL,
    source_key          TEXT NOT NULL,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (meeting_id, lang)
);
