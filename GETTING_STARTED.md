# Getting started — New Dawn

_The scaffold was generated on 2026-10-23. This file lists what is already in place and the steps only you can do on your Mac before the first Claude Code session._

## What is already in place
- Tauri 2 + React 19 + TypeScript + Vite scaffold (`create-tauri-app`, template react-ts, pnpm).
- Identity: product name **New Dawn**, bundle id **`co.majorisdigital.newdawn`** (`src-tauri/tauri.conf.json`). Do not change the bundle id after the first run.
- Window pre-configured for the glass shell: transparent, overlay title bar, hidden title, traffic lights inset, min 960×600, CSP that allows only `localhost:11434` (Ollama).
- `src-tauri/.cargo/config.toml` — macOS 14.2 deployment target (needed for system-audio capture).
- `src-tauri/Info.plist` (microphone + system-audio usage descriptions) and `src-tauri/entitlements.plist` (`device.audio-input` only).
- Be Vietnam Pro wired in (`@fontsource/be-vietnam-pro` in `package.json`, imported in `src/main.tsx`), design tokens in `src/styles/tokens.css`, base styles in `src/styles/base.css`, placeholder `src/App.tsx`.
- Claude Code control files: `CLAUDE.md`, `DECISIONS.md` (D-001…D-005), `LAST_BUILD.md`, `.claude/agents/{architect-reviewer,migration-safety}.md`, `.claude/commands/build-check.md`, `.claude/skills/newdawn-ui-patterns/SKILL.md`.
- `docs/` — `plan.md`, `ai-engine.md`, `meetily-teardown.md`, `reference-audit.md`, `playbook.md` (all the build prompts), `journal.md`.
- `brands/vifc/` (draft tokens + README; **logo folder empty until you copy the SVGs**) and `brands/default/`.
- `scripts/check-localhost-only.sh`, empty `src-tauri/migrations/{registry,workspace}/`, `schemas/`.

## Steps only you can do (Terminal, in this order)

```bash
# 1. Apple tools — install Xcode 27 from the App Store first, then:
xcode-select --install
sudo xcodebuild -license accept

# 2. Homebrew (if missing) and packages
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew install cmake pnpm ollama

# 3. Rust toolchain + Tauri/sqlx CLIs
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh      # accept defaults
source "$HOME/.cargo/env"
cargo install tauri-cli --version "^2"
cargo install sqlx-cli --no-default-features --features sqlite

# 4. Local models (pull once; try one to feel the speed on the M1 Pro)
ollama pull qwen3.5:9b
ollama pull qwen3.5:4b
ollama run qwen3.5:9b "Say hello in one line."

# 5. Reference code next to the old projects (Claude Code reads it when porting audio)
cd "$HOME/Astral Archive/SEED/Ref"
git clone --depth 1 https://github.com/Zackriya-Solutions/meetily meetily
rm -rf "$HOME/Astral Archive/SEED/_to_delete"      # broken partial clone, safe to remove

# 6. This project
cd "$HOME/Astral Archive/SEED/new-dawn"
pnpm install
pnpm tauri dev          # first build takes a few minutes; a window titled New Dawn must open
# Close it, then:
git init && git add -A && git commit -m "chore: scaffold New Dawn"
```

7. Copy the four VIFC logo SVGs from OneDrive → "VIFC Space / Brand Guideline / Logo" into `brands/vifc/logo/` (names listed in `brands/vifc/README.md`).

## First Claude Code session

Open this folder in Claude Code and paste **P0** from `docs/playbook.md` §4. Most of P0's file changes are already applied; Claude Code will verify them, fill any gaps, and run `/build-check`. Then continue with **P1.1** (the native Liquid Glass window).

## Daily rhythm
One prompt per sitting · ALIGN before BUILD · `cargo check` → `pnpm tauri dev` → `/build-check` → update `LAST_BUILD.md` → commit · write a line in `docs/journal.md`.
