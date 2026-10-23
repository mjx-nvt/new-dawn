---
description: Pre-commit checklist for New Dawn (type-check, lint, localhost-only, bounded channels, docs updated)
---

Run the New Dawn pre-commit checklist. For each item run the command, then report ✅ pass / ⚠️ non-blocking / ❌ must fix with a one-line reason:

1. `cd src-tauri && cargo check` — must be clean.
2. `pnpm tsc --noEmit` — must be clean.
3. `cd src-tauri && cargo clippy -- -D warnings` — report warnings; blocking only for `unwrap()` on user-facing paths and for `unbounded_channel`.
4. `bash scripts/check-localhost-only.sh` — must pass (no non-localhost URLs in `src-tauri/src` or `src`).
5. Grep `src-tauri/src` for `unbounded_channel` — must be zero hits.
6. Grep `src` for `invoke(` outside `src/kernel/bridge/` — must be zero hits (UI uses the typed bridge).
7. Read `LAST_BUILD.md` — must describe this session's changes and list manual steps; ❌ if stale.
8. If any file under `src-tauri/migrations/` changed this session, confirm the `migration-safety` agent was run; ❌ if not.
9. List any new `TODO:` / `FIXME:` introduced this session.

End with exactly one line: "Ready to commit" or "Fix these N issues before commit: …".
