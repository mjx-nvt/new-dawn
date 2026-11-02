-- 0.10.14 (D-071): the local REST API's keys, and webhooks to this Mac with their deliveries.
-- Whether the API is on and its port are the app's (registry settings), not a workspace's.

CREATE TABLE api_key (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    -- The key's first characters, shown in the list; the key itself is shown once, never kept.
    prefix        TEXT NOT NULL,
    -- SHA-256 of the whole key, hex: what a request's key is checked against.
    key_hash      TEXT NOT NULL UNIQUE,
    expires_at    TEXT,
    revoked_at    TEXT,
    last_used_at  TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE webhook (
    id           TEXT PRIMARY KEY,
    -- An address on this Mac only (localhost, 127.0.0.1, [::1]); the code refuses any other.
    url          TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    -- The events it gets: a JSON array of "<object>.<created|updated|deleted>", "<object>.*", "*.*".
    events_json  TEXT NOT NULL DEFAULT '["*.*"]',
    -- The key of its HMAC-SHA256 signature (X-Twenty-Webhook-Signature).
    secret       TEXT NOT NULL,
    is_active    INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE webhook_delivery (
    id           TEXT PRIMARY KEY,
    webhook_id   TEXT NOT NULL REFERENCES webhook (id) ON DELETE CASCADE,
    event        TEXT NOT NULL,
    record_id    TEXT NOT NULL DEFAULT '',
    status_code  INTEGER,
    ok           INTEGER NOT NULL DEFAULT 0 CHECK (ok IN (0, 1)),
    error        TEXT NOT NULL DEFAULT '',
    duration_ms  INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
CREATE INDEX idx_webhook_delivery_webhook ON webhook_delivery (webhook_id, created_at);
