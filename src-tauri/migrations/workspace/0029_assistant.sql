-- 0.10.13 (D-070): the CRM's AI assistant — its agents (settings, not CRM records, like
-- Twenty's), chat threads (one may sit on a record) and their messages, with the writes an
-- agent proposes for the owner to apply. Named crm_* so they never meet the meeting agents'
-- guides and logs (D-035).

CREATE TABLE crm_agent (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    instructions  TEXT NOT NULL DEFAULT '',
    -- '' = the app's selected model
    model         TEXT NOT NULL DEFAULT '',
    -- the tools it may use: a JSON array of names
    tools_json    TEXT NOT NULL DEFAULT '[]',
    is_default    INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
    position      REAL NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
-- One default agent a workspace (made the first time the assistant is asked).
CREATE UNIQUE INDEX idx_crm_agent_default ON crm_agent (is_default) WHERE is_default = 1;

CREATE TABLE crm_chat_thread (
    id             TEXT PRIMARY KEY,
    agent_id       TEXT REFERENCES crm_agent (id) ON DELETE SET NULL,
    title          TEXT NOT NULL DEFAULT '',
    -- A thread on a record: its object (singular name) and id; empty for a thread of its own.
    record_object  TEXT NOT NULL DEFAULT '',
    record_id      TEXT NOT NULL DEFAULT '',
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_crm_chat_thread_updated ON crm_chat_thread (updated_at);
CREATE INDEX idx_crm_chat_thread_record ON crm_chat_thread (record_object, record_id);
CREATE INDEX idx_crm_chat_thread_agent ON crm_chat_thread (agent_id);

CREATE TABLE crm_chat_message (
    id               TEXT PRIMARY KEY,
    thread_id        TEXT NOT NULL REFERENCES crm_chat_thread (id) ON DELETE CASCADE,
    -- user · assistant · tool
    role             TEXT NOT NULL,
    content          TEXT NOT NULL DEFAULT '',
    -- assistant: the tool calls it made (JSON); tool: the result it gave (JSON)
    tool_name        TEXT NOT NULL DEFAULT '',
    tool_json        TEXT NOT NULL DEFAULT '',
    -- A write the agent proposes ({kind, object, id, fields}); only the owner applies it.
    proposal_json    TEXT NOT NULL DEFAULT '',
    -- '' · PROPOSED · APPLIED · DISMISSED
    proposal_status  TEXT NOT NULL DEFAULT '',
    -- What applying made (the record's id), or why it couldn't.
    proposal_result  TEXT NOT NULL DEFAULT '',
    model            TEXT NOT NULL DEFAULT '',
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_crm_chat_message_thread ON crm_chat_message (thread_id, created_at);

-- A thread's time is its last message's, so the list follows the latest conversation.
CREATE TRIGGER crm_chat_message_touch_thread AFTER INSERT ON crm_chat_message BEGIN
    UPDATE crm_chat_thread SET updated_at = new.created_at WHERE id = new.thread_id;
END;
