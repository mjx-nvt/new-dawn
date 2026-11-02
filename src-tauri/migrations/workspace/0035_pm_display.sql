-- 0.11.3 (D-076): a project's display (Plane's layouts and display options), remembered per
-- project: one owner, where Plane keeps them per user. No row: the defaults (PmDisplay's,
-- mirrored below). Enumerations are checked in Rust.
--   layout                  list | board | calendar | table | timeline
--   group_by, sub_group_by  state | group | priority | labels | assignees | created_by | none
--   order_by                manual | created | updated | start | due | priority
--   properties_json         the properties a row, a card or the table shows, a JSON array
--   calendar                month | week;   zoom (the timeline's)  week | month | quarter
--   collapsed_json          the Board's folded columns, a JSON array of their group keys —
--                           grouped by assignees, "me" or a person's id

CREATE TABLE pm_display (
    project_id       TEXT NOT NULL PRIMARY KEY REFERENCES pm_project (project_id) ON DELETE CASCADE,
    layout           TEXT NOT NULL DEFAULT 'list',
    group_by         TEXT NOT NULL DEFAULT 'state',
    sub_group_by     TEXT NOT NULL DEFAULT 'none',
    order_by         TEXT NOT NULL DEFAULT 'manual',
    properties_json  TEXT NOT NULL DEFAULT '["id","state","priority","assignees","labels","due","sub_items"]',
    sub_items        INTEGER NOT NULL DEFAULT 1 CHECK (sub_items IN (0, 1)),
    empty_groups     INTEGER NOT NULL DEFAULT 1 CHECK (empty_groups IN (0, 1)),
    calendar         TEXT NOT NULL DEFAULT 'month',
    zoom             TEXT NOT NULL DEFAULT 'month',
    collapsed_json   TEXT NOT NULL DEFAULT '[]',
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- A person deleted for good is no folded column any more.
CREATE TRIGGER person_ad_pm_display AFTER DELETE ON person BEGIN
    UPDATE pm_display
       SET collapsed_json = (SELECT json_group_array(c.value) FROM json_each(pm_display.collapsed_json) AS c WHERE c.value <> old.id)
     WHERE EXISTS (SELECT 1 FROM json_each(pm_display.collapsed_json) AS c WHERE c.value = old.id);
END;
