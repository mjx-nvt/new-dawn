-- The naming agent's pending suspects for a summary (JSON; NULL once the owner confirmed or
-- dismissed them). Additive only.

ALTER TABLE summary ADD COLUMN naming_json TEXT;
