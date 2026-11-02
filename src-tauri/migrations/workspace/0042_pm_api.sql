-- 0.11.11 (D-084): Plane's API tokens and webhooks, on the local API's keys and webhooks (0030).
-- One key store opens both path families (Twenty's /rest, Plane's /api/v1); Plane's tokens
-- have a description.
ALTER TABLE api_key ADD COLUMN description TEXT NOT NULL DEFAULT '';

-- The API a webhook speaks: Twenty's record events (0.10.14) or Plane's project, work item,
-- cycle, module and comment events. Every webhook made before this one is Twenty's.
ALTER TABLE webhook ADD COLUMN format TEXT NOT NULL DEFAULT 'twenty' CHECK (format IN ('twenty', 'plane'));
