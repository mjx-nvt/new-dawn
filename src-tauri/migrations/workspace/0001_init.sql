-- Workspace (tenant) schema v1 — docs/plan.md §6. Applied to every workspace.db.

CREATE TABLE document_template (
    id            TEXT PRIMARY KEY,               -- UUID v7
    kind          TEXT NOT NULL,                  -- meeting-minutes | partner-brief | …
    name          TEXT NOT NULL,
    folder        TEXT NOT NULL,                  -- relative to the workspace's brand/templates/
    manifest_json TEXT NOT NULL DEFAULT '{}',
    is_default    INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_document_template_kind ON document_template (kind);
CREATE UNIQUE INDEX idx_document_template_one_default ON document_template (kind) WHERE is_default = 1;

CREATE TABLE meeting_type (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL,
    language               TEXT NOT NULL DEFAULT 'en',
    template_id            TEXT REFERENCES document_template (id) ON DELETE SET NULL,
    summary_structure_json TEXT NOT NULL DEFAULT '[]',   -- ordered sections with instructions
    email_format_json      TEXT NOT NULL DEFAULT '{}',   -- subject pattern, greeting, sections, sign-off
    include_transcript     INTEGER NOT NULL DEFAULT 0 CHECK (include_transcript IN (0, 1)),
    is_default             INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    created_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at             TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_meeting_type_template ON meeting_type (template_id);
CREATE UNIQUE INDEX idx_meeting_type_one_default ON meeting_type (is_default) WHERE is_default = 1;

CREATE TABLE meeting (
    id              TEXT PRIMARY KEY,
    meeting_type_id TEXT REFERENCES meeting_type (id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    held_on         TEXT NOT NULL,                -- ISO-8601 UTC start time
    language        TEXT NOT NULL DEFAULT 'en',
    audio_path      TEXT,
    duration_ms     INTEGER,
    status          TEXT NOT NULL DEFAULT 'recording' CHECK (status IN
                    ('recording', 'recorded', 'transcribing', 'transcribed', 'summarising', 'summarised', 'failed')),
    notes_md        TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_meeting_type_id ON meeting (meeting_type_id);
CREATE INDEX idx_meeting_held_on ON meeting (held_on);
CREATE INDEX idx_meeting_status ON meeting (status);

CREATE TABLE transcript_segment (
    id         TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL REFERENCES meeting (id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    start_ms   INTEGER NOT NULL,
    end_ms     INTEGER NOT NULL,
    text       TEXT NOT NULL,
    fts_rowid  INTEGER,                         -- set by trigger; links to transcript_fts
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE (meeting_id, seq)
);

CREATE TABLE summary (
    id            TEXT PRIMARY KEY,
    meeting_id    TEXT NOT NULL UNIQUE REFERENCES meeting (id) ON DELETE CASCADE,
    sections_json TEXT NOT NULL DEFAULT '[]',
    email_subject TEXT,
    email_md      TEXT,
    model         TEXT,
    fts_rowid     INTEGER,                      -- set by trigger; links to summary_fts
    edited_at     TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE document_export (
    id          TEXT PRIMARY KEY,
    meeting_id  TEXT NOT NULL REFERENCES meeting (id) ON DELETE CASCADE,
    template_id TEXT REFERENCES document_template (id) ON DELETE SET NULL,
    docx_path   TEXT,
    pdf_path    TEXT,
    converter   TEXT,                             -- word | libreoffice | pages
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_document_export_meeting ON document_export (meeting_id);
CREATE INDEX idx_document_export_template ON document_export (template_id);

CREATE TABLE ai_log (
    id            TEXT PRIMARY KEY,
    kind          TEXT NOT NULL CHECK (kind IN ('transcribe', 'complete')),
    model         TEXT NOT NULL,
    meeting_id    TEXT REFERENCES meeting (id) ON DELETE SET NULL,
    prompt_tokens INTEGER,
    output_tokens INTEGER,
    duration_ms   INTEGER,
    status        TEXT NOT NULL CHECK (status IN ('ok', 'error', 'cancelled')),
    error         TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_ai_log_created ON ai_log (created_at);
CREATE INDEX idx_ai_log_meeting ON ai_log (meeting_id);

-- Key/value; the key is the identity, so no separate UUID.
CREATE TABLE setting (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Full-text search. The FTS tables hold their own copy of the text: the source tables have TEXT
-- primary keys, so their implicit rowid may change on VACUUM and cannot be an external-content
-- link. Instead each source row stores its FTS rowid (stable: FTS5 shadow tables use INTEGER
-- PRIMARY KEY), so sync triggers use an indexed rowid lookup rather than a scan.

CREATE VIRTUAL TABLE transcript_fts USING fts5 (
    text,
    segment_id UNINDEXED,
    meeting_id UNINDEXED
);

CREATE TRIGGER transcript_segment_ai AFTER INSERT ON transcript_segment BEGIN
    INSERT INTO transcript_fts (text, segment_id, meeting_id) VALUES (new.text, new.id, new.meeting_id);
    UPDATE transcript_segment SET fts_rowid = last_insert_rowid() WHERE id = new.id;
END;

CREATE TRIGGER transcript_segment_ad AFTER DELETE ON transcript_segment BEGIN
    DELETE FROM transcript_fts WHERE rowid = old.fts_rowid;
END;

CREATE TRIGGER transcript_segment_au AFTER UPDATE OF text, meeting_id ON transcript_segment BEGIN
    UPDATE transcript_fts SET text = new.text, meeting_id = new.meeting_id WHERE rowid = old.fts_rowid;
END;

CREATE VIRTUAL TABLE summary_fts USING fts5 (
    email_subject,
    email_md,
    summary_id UNINDEXED,
    meeting_id UNINDEXED
);

CREATE TRIGGER summary_ai AFTER INSERT ON summary BEGIN
    INSERT INTO summary_fts (email_subject, email_md, summary_id, meeting_id)
    VALUES (coalesce(new.email_subject, ''), coalesce(new.email_md, ''), new.id, new.meeting_id);
    UPDATE summary SET fts_rowid = last_insert_rowid() WHERE id = new.id;
END;

CREATE TRIGGER summary_ad AFTER DELETE ON summary BEGIN
    DELETE FROM summary_fts WHERE rowid = old.fts_rowid;
END;

CREATE TRIGGER summary_au AFTER UPDATE OF email_subject, email_md, meeting_id ON summary BEGIN
    UPDATE summary_fts
    SET email_subject = coalesce(new.email_subject, ''), email_md = coalesce(new.email_md, ''), meeting_id = new.meeting_id
    WHERE rowid = old.fts_rowid;
END;
