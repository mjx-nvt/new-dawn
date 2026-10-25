-- People memory (D-028): the correct name, job title and organisation of people this workspace
-- meets, plus other spellings the transcript or the model produced. Used to correct attendees
-- after every summary and given to the model as known spellings. Additive only.

CREATE TABLE person (
    id           TEXT PRIMARY KEY,                -- UUID v7
    name         TEXT NOT NULL,
    title        TEXT NOT NULL DEFAULT '',         -- job title
    organisation TEXT NOT NULL DEFAULT '',
    aliases_json TEXT NOT NULL DEFAULT '[]',       -- other spellings, JSON array of strings
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX idx_person_name ON person (name COLLATE NOCASE);
