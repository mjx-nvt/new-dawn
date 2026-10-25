-- The owner's job title in each workspace (D-039). The owner's name is app-wide (setting
-- `owner_name`); each workspace adds the title the owner holds there. Additive only.
ALTER TABLE workspace ADD COLUMN owner_title TEXT NOT NULL DEFAULT '';
