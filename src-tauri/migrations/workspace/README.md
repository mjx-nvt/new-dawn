Workspace migrations — applied to EVERY workspace.db on open (projects, tasks, notes, meetings, entities, research, ai_log, embeddings, FTS5).
Add with: sqlx migrate add <name> --source src-tauri/migrations/workspace
Append-only. Review with the migration-safety agent before running.
