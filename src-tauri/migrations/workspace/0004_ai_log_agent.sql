-- Which agent made an AI call (summary | naming | email | minutes), so Workspace → Agents can
-- show each agent's last run. Additive only; older rows stay NULL.

ALTER TABLE ai_log ADD COLUMN agent TEXT;
CREATE INDEX idx_ai_log_agent ON ai_log (agent, created_at);
