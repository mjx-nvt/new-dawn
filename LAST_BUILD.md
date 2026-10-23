# Last Build — Scaffold (pre-P0)
_2026-10-23_

## Summary
Project scaffolded with `create-tauri-app` (react-ts, pnpm) and overlaid with the New Dawn control files: `CLAUDE.md`, `DECISIONS.md` (D-001…D-005), `.claude/` agents, command and UI skill, `docs/` (plan, AI engine, Meetily teardown, reference audit, playbook, journal), `brands/` (vifc + default), base styles with Be Vietnam Pro, Tauri identity (`co.majorisdigital.newdawn`, "New Dawn"), `.cargo/config.toml` (macOS 14.2 target), `Info.plist` + `entitlements.plist` for microphone and system-audio capture, and `scripts/check-localhost-only.sh`.

No features built. Dependencies not installed yet.

## Files created / modified
- Scaffold: `package.json`, `index.html`, `vite.config.ts`, `tsconfig*.json`, `src/*`, `src-tauri/*` (from create-tauri-app)
- Overlay: see `GETTING_STARTED.md` §"What is already in place"

## Build state
- `cargo check`: not yet run (Rust not installed at scaffold time)
- `pnpm tsc --noEmit`: not yet run (dependencies not installed)
- `pnpm tauri dev`: not yet run

## Manual steps required (see GETTING_STARTED.md)
1. Install Xcode 27 + Command Line Tools, Homebrew, CMake, Rust (rustup), tauri-cli, sqlx-cli, pnpm, Ollama.
2. `pnpm install` in this folder, then `pnpm tauri dev` — the hello window must open in Be Vietnam Pro.
3. `git init && git add -A && git commit -m "chore: scaffold New Dawn"`.
4. Clone Meetily into `../Ref/meetily`; copy the VIFC logo SVGs into `brands/vifc/logo/`.
5. Open the folder in Claude Code and run prompt **P0** from `docs/playbook.md` (verification mode — most of its steps are already applied).

## Open items
- `brands/vifc/logo/` is empty until the SVGs are copied from OneDrive.
- `bundle.macOS.minimumSystemVersion` is set to 14.2 (audio taps); Liquid Glass requires 26+ at runtime and falls back to vibrancy below that.
