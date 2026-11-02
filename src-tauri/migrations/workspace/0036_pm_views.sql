-- 0.11.4 (D-077): Plane's views — saved filters with a layout and its display. A project's view
-- (project_id) or the workspace's (none: across its projects); private and locked are flags (one
-- owner). Enumerations, the rules' fields and operators are checked in Rust.
--   rules_json     the filters, a JSON array of {field, op, values[]} (nested: a rule's values);
--                  assignees, created_by, subscribers and mentions hold "me" or person ids
--   layout, group_by, sub_group_by, order_by, properties_json, sub_items, empty_groups,
--   calendar, zoom as pm_display's (0035); a view doesn't keep a board's folded columns

CREATE TABLE pm_view (
    id               TEXT NOT NULL PRIMARY KEY,
    project_id       TEXT REFERENCES pm_project (project_id) ON DELETE CASCADE,
    name             TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    rules_json       TEXT NOT NULL DEFAULT '[]' CHECK (json_valid(rules_json) AND json_type(rules_json) = 'array'),
    layout           TEXT NOT NULL DEFAULT 'list',
    group_by         TEXT NOT NULL DEFAULT 'state',
    sub_group_by     TEXT NOT NULL DEFAULT 'none',
    order_by         TEXT NOT NULL DEFAULT 'manual',
    properties_json  TEXT NOT NULL DEFAULT '["id","state","priority","assignees","labels","due","sub_items"]',
    sub_items        INTEGER NOT NULL DEFAULT 1 CHECK (sub_items IN (0, 1)),
    empty_groups     INTEGER NOT NULL DEFAULT 1 CHECK (empty_groups IN (0, 1)),
    calendar         TEXT NOT NULL DEFAULT 'month',
    zoom             TEXT NOT NULL DEFAULT 'month',
    is_private       INTEGER NOT NULL DEFAULT 0 CHECK (is_private IN (0, 1)),
    is_locked        INTEGER NOT NULL DEFAULT 0 CHECK (is_locked IN (0, 1)),
    sort_order       REAL NOT NULL DEFAULT 65535,
    created_by       TEXT NOT NULL DEFAULT 'me',
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_pm_view_project ON pm_view (project_id, sort_order);

-- The sidebar's Favorites can hold a view (D-064's favorites).
ALTER TABLE favorite ADD COLUMN pm_view_id TEXT REFERENCES pm_view (id) ON DELETE CASCADE;
CREATE INDEX idx_favorite_pm_view ON favorite (pm_view_id);

-- A person deleted for good leaves no value in a view's people rules.
CREATE TRIGGER person_ad_pm_view AFTER DELETE ON person BEGIN
    UPDATE pm_view
       SET rules_json = (
           SELECT json_group_array(json(CASE
               WHEN json_extract(r.value, '$.field') IN ('assignees', 'created_by', 'subscribers', 'mentions')
               THEN json_set(r.value, '$.values', json((SELECT json_group_array(v.value) FROM json_each(r.value, '$.values') AS v WHERE v.value <> old.id)))
               ELSE r.value END))
           FROM json_each(pm_view.rules_json) AS r)
     WHERE EXISTS (SELECT 1 FROM json_each(pm_view.rules_json) AS r, json_each(r.value, '$.values') AS v
                   WHERE json_extract(r.value, '$.field') IN ('assignees', 'created_by', 'subscribers', 'mentions') AND v.value = old.id);
END;
