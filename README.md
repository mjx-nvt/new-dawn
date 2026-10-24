# New Dawn

A local-first, native macOS personal workspace for people who work across several companies at once.

- **One app, many workspaces** — each company or project gets its own SQLite database and its own brand pack (logo, colours, corner radius, buttons). Each workspace also defines its meeting types.
- **Looks native** — real Liquid Glass shell, Be Vietnam Pro everywhere, dark mode, system accent colour.
- **100 % local AI** — Whisper for speech, Qwen via Ollama for text. No cloud, no API keys, no network beyond `localhost`.
- **Records meetings itself** — microphone plus the call's system audio, no bot in the call; full transcript after Stop, an executive summary for email, and branded meeting-minutes DOCX/PDF.

Built with Tauri 2 (Rust engine) + React/TypeScript (cockpit).

## Start here
1. `GETTING_STARTED.md` — the setup steps on your Mac.
2. `CLAUDE.md` — the rules every Claude Code session follows.
3. `docs/playbook.md` — the build plan and the prompt for each session.
4. `docs/plan.md`, `docs/ai-engine.md`, `docs/meetily-teardown.md` — the design and the reference teardown.

## Commands
```bash
pnpm install          # once
pnpm tauri dev        # run the app in development
pnpm typecheck        # TypeScript
cd src-tauri && cargo check && cargo clippy   # Rust
pnpm check:localhost  # rule 3: no network except localhost
pnpm tauri build      # New Dawn.app + .dmg (Phase 4)
```

## Layout
```
CLAUDE.md DECISIONS.md LAST_BUILD.md GETTING_STARTED.md
.claude/     agents, commands, skills for Claude Code
docs/        plan · ai-engine · meetily-teardown · reference-audit · playbook · journal
brands/      starter brand packs (vifc, default)
schemas/     dossier schemas (later)
scripts/     check-localhost-only.sh
src/         React cockpit (kernel/, modules/, styles/)
src-tauri/   Rust engine (src/kernel, src/recorder, src/modules, src/commands, migrations/)
```
