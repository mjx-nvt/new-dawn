-- Accent-insensitive search for Vietnamese (user-approved, D-022).
-- 1. `remove_diacritics 2`: the default unicode61 tokenizer (remove_diacritics 1) folds only
--    letters with one diacritic; ế, ộ, ữ… carry two. With 2, "tieng" finds "Tiếng".
-- 2. `folded` column: đ/Đ is its own letter in Unicode (not d + a mark), so no tokenizer option
--    folds it. The triggers store a copy with đ→d, Đ→D; MATCH searches every column, while
--    snippets keep using the original column so they show the real spelling.
--
-- Derived data only: both FTS tables and their sync triggers are rebuilt from the source rows in
-- this migration; nothing the user created is lost. Each FTS row takes its source row's rowid at
-- rebuild time and `fts_rowid` is set to match; later inserts get new FTS rowids from the triggers.

DROP TRIGGER transcript_segment_ai;
DROP TRIGGER transcript_segment_ad;
DROP TRIGGER transcript_segment_au;
DROP TABLE transcript_fts;

CREATE VIRTUAL TABLE transcript_fts USING fts5 (
    text,
    folded,
    segment_id UNINDEXED,
    meeting_id UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

INSERT INTO transcript_fts (rowid, text, folded, segment_id, meeting_id)
SELECT rowid, text, replace(replace(text, 'đ', 'd'), 'Đ', 'D'), id, meeting_id FROM transcript_segment;

UPDATE transcript_segment SET fts_rowid = rowid;

CREATE TRIGGER transcript_segment_ai AFTER INSERT ON transcript_segment BEGIN
    INSERT INTO transcript_fts (text, folded, segment_id, meeting_id)
    VALUES (new.text, replace(replace(new.text, 'đ', 'd'), 'Đ', 'D'), new.id, new.meeting_id);
    UPDATE transcript_segment SET fts_rowid = last_insert_rowid() WHERE id = new.id;
END;

CREATE TRIGGER transcript_segment_ad AFTER DELETE ON transcript_segment BEGIN
    DELETE FROM transcript_fts WHERE rowid = old.fts_rowid;
END;

CREATE TRIGGER transcript_segment_au AFTER UPDATE OF text, meeting_id ON transcript_segment BEGIN
    UPDATE transcript_fts
    SET text = new.text, folded = replace(replace(new.text, 'đ', 'd'), 'Đ', 'D'), meeting_id = new.meeting_id
    WHERE rowid = old.fts_rowid;
END;

DROP TRIGGER summary_ai;
DROP TRIGGER summary_ad;
DROP TRIGGER summary_au;
DROP TABLE summary_fts;

CREATE VIRTUAL TABLE summary_fts USING fts5 (
    email_subject,
    email_md,
    folded,
    summary_id UNINDEXED,
    meeting_id UNINDEXED,
    tokenize = 'unicode61 remove_diacritics 2'
);

INSERT INTO summary_fts (rowid, email_subject, email_md, folded, summary_id, meeting_id)
SELECT rowid, coalesce(email_subject, ''), coalesce(email_md, ''),
       replace(replace(coalesce(email_subject, '') || ' ' || coalesce(email_md, ''), 'đ', 'd'), 'Đ', 'D'),
       id, meeting_id
FROM summary;

UPDATE summary SET fts_rowid = rowid;

CREATE TRIGGER summary_ai AFTER INSERT ON summary BEGIN
    INSERT INTO summary_fts (email_subject, email_md, folded, summary_id, meeting_id)
    VALUES (coalesce(new.email_subject, ''), coalesce(new.email_md, ''),
            replace(replace(coalesce(new.email_subject, '') || ' ' || coalesce(new.email_md, ''), 'đ', 'd'), 'Đ', 'D'),
            new.id, new.meeting_id);
    UPDATE summary SET fts_rowid = last_insert_rowid() WHERE id = new.id;
END;

CREATE TRIGGER summary_ad AFTER DELETE ON summary BEGIN
    DELETE FROM summary_fts WHERE rowid = old.fts_rowid;
END;

CREATE TRIGGER summary_au AFTER UPDATE OF email_subject, email_md, meeting_id ON summary BEGIN
    UPDATE summary_fts
    SET email_subject = coalesce(new.email_subject, ''), email_md = coalesce(new.email_md, ''),
        folded = replace(replace(coalesce(new.email_subject, '') || ' ' || coalesce(new.email_md, ''), 'đ', 'd'), 'Đ', 'D'),
        meeting_id = new.meeting_id
    WHERE rowid = old.fts_rowid;
END;
