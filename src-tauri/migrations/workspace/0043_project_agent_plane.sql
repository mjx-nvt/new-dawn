-- 0.11.12 (D-085): the Projects agent proposes on Plane's model — a new work item, a state for
-- one, a comment on one — as rows of the dossier's proposals (0016 `project_update`, D-054),
-- applied or dismissed the same way. Their `field`: work_item, item_state, comment.

-- The work item a proposal is about (a state, a comment), or the one it made once applied.
ALTER TABLE project_update ADD COLUMN item_id TEXT REFERENCES pm_item (id) ON DELETE SET NULL;

-- A new work item's fields (assignees, dates, priority, labels, parent, cycle) or a state's id,
-- as JSON; NULL for the dossier's own fields.
ALTER TABLE project_update ADD COLUMN payload_json TEXT;

CREATE INDEX idx_project_update_item ON project_update (item_id);

-- Before a work item goes: proposals about it still waiting go with it; decided ones stay
-- (applied = the history, dismissed = the agent's record), unlinked by the SET NULL. BEFORE,
-- because SQLite runs foreign-key actions ahead of AFTER triggers (as 0018's email_item_bd).
CREATE TRIGGER pm_item_bd_project_update BEFORE DELETE ON pm_item BEGIN
    DELETE FROM project_update WHERE item_id = old.id AND status = 'proposed';
END;
