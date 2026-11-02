-- 0.11.5 (D-078): Plane's cycles — a project's time-boxed runs of work: both dates or none (a
-- draft), never two at once in a project (checked in Rust), their status read from the dates
-- (or completed early), the open work moved on once, archived when completed. An item is in one
-- cycle; a cycle deleted leaves its items, in none. created_by holds who acted ("me", an agent,
-- an automation) — no person ids.

CREATE TABLE pm_cycle (
    id              TEXT NOT NULL PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES pm_project (project_id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    start_date      TEXT,
    end_date        TEXT,
    completed_at    TEXT,
    transferred_at  TEXT,
    -- The numbers as they stood when the open work moved on (Plane's progress snapshot); NULL before.
    progress_json   TEXT CHECK (progress_json IS NULL OR json_valid(progress_json)),
    archived_at     TEXT,
    is_favorite     INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
    created_by      TEXT NOT NULL DEFAULT 'me',
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    CHECK ((start_date IS NULL) = (end_date IS NULL)),
    -- Days as YYYY-MM-DD: their text order is the days' order (below, and the overlap and active queries).
    CHECK (date(start_date) IS start_date AND date(end_date) IS end_date),
    CHECK (start_date IS NULL OR start_date <= end_date)
);

CREATE INDEX idx_pm_cycle_project ON pm_cycle (project_id, start_date);

-- One cycle an item (the item is the key).
CREATE TABLE pm_cycle_item (
    item_id     TEXT NOT NULL PRIMARY KEY REFERENCES pm_item (id) ON DELETE CASCADE,
    cycle_id    TEXT NOT NULL REFERENCES pm_cycle (id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_cycle_item_cycle ON pm_cycle_item (cycle_id);
