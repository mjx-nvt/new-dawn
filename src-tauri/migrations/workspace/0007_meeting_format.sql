-- Meeting format (D-038): a discussion among several people, or a presentation with one main
-- presenter (who may never be named in the audio). Guides Whisper (the presenter's name) and
-- the summary (who said what). Additive only.

ALTER TABLE meeting ADD COLUMN format TEXT NOT NULL DEFAULT 'discussion' CHECK (format IN ('discussion', 'presentation'));
ALTER TABLE meeting ADD COLUMN presenter TEXT;
