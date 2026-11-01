-- 0.10.8 (D-065). Additive only: what was imported into and exported from the record engine.

-- One row per import run: the object, the file, and the counts of the report.
CREATE TABLE import_job (
    id              TEXT PRIMARY KEY,
    object          TEXT NOT NULL,
    file_name       TEXT NOT NULL DEFAULT '',
    rows_total      INTEGER NOT NULL DEFAULT 0,
    created_count   INTEGER NOT NULL DEFAULT 0,
    updated_count   INTEGER NOT NULL DEFAULT 0,
    skipped_count   INTEGER NOT NULL DEFAULT 0,
    errors_json     TEXT NOT NULL DEFAULT '[]',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_import_job_created_at ON import_job (created_at);

-- One row per export: the object, the view, where the file went and how many rows it holds.
CREATE TABLE export_job (
    id              TEXT PRIMARY KEY,
    object          TEXT NOT NULL,
    view_name       TEXT NOT NULL DEFAULT '',
    file_path       TEXT NOT NULL DEFAULT '',
    row_count       INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_export_job_created_at ON export_job (created_at);
