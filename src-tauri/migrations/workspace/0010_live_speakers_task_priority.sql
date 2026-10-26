-- 0.7.5 (D-043). Additive only: new columns with defaults or NULL; existing rows unchanged.

-- Task priority, as in Apple Reminders: 0 none, 1 low (!), 2 medium (!!), 3 high (!!!).
ALTER TABLE task ADD COLUMN priority INTEGER NOT NULL DEFAULT 0 CHECK (priority IN (0, 1, 2, 3));

-- Live lines. Who spoke: 'you' = this Mac's microphone, 'them' = the call's audio; NULL = not
-- known (batch transcripts, presentations, a meeting in a room).
ALTER TABLE transcript_segment ADD COLUMN speaker TEXT CHECK (speaker IN ('you', 'them'));

-- How clearly Whisper heard the line: 'uncertain' (translated with a marker) or 'unclear' (not
-- translated: several voices at once or a weak signal); NULL = clear, or not scored (batch).
ALTER TABLE transcript_segment ADD COLUMN clarity TEXT CHECK (clarity IN ('uncertain', 'unclear'));

-- The line's language when a live session translates both ways ('vi' | 'en' today, checked in
-- Rust like meeting.language, so a new language needs no table rebuild); NULL = the meeting's.
ALTER TABLE transcript_segment ADD COLUMN lang TEXT;
