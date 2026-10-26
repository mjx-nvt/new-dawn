-- Live translation (D-041): the English of a line written during a live session, beside its
-- Vietnamese `text` (NULL = not translated). Additive only. The FTS triggers index `text` and fire
-- on `UPDATE OF text, meeting_id`, so setting a translation never touches transcript_fts.

ALTER TABLE transcript_segment ADD COLUMN translation TEXT;
