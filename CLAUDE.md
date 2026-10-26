# New Dawn

Native macOS personal workspace for a professional who works across several companies. One app, many **tenants / workspaces** (one per company), each with its own SQLite database, brand pack and meeting types. **v1 is the meeting machine**: record a call (mic + system audio, no bot) → full transcript after Stop → detailed executive summary (email body) → designed meeting-minutes DOCX/PDF in the tenant's brand. **All AI runs locally** (Whisper for speech, Qwen via Ollama for text), kept simple: one speech model, one text model, never concurrent — except a **Live** session (0.7.2, D-041), which keeps Whisper and the fast text model loaded together and holds the AI queue until Stop. A simple **Tasks** list (0.7.3, D-042) is fed by meeting action items; the cockpit, notes and knowledge come later.

This file is the source of truth for how to work on this codebase. Read it at the start of every session, then `docs/plan.md` (v2). For AI features also read `docs/ai-engine.md`; when porting audio/transcription read `docs/meetily-teardown.md` and the reference code at `../Ref/meetily`; for the minutes document read `docs/document-export.md`. The build prompts live in `docs/playbook.md` (v2).

---

## Stack
- **App**: Tauri 2 (Rust engine) + React 19 + TypeScript strict + Vite. pnpm.
- **UI styling**: plain CSS modules + CSS variables. NO Tailwind, NO CSS-in-JS.
- **Typeface**: DM Sans (Majoris; bundled via `@fontsource-variable/dm-sans`) for everything inside the window; never introduce another UI typeface. A tenant's `document_fonts` (heading/body) apply **only to exported documents**, never to the UI.
- **Native look**: real Liquid Glass shell on macOS 26+ via the `window-vibrancy` crate (`apply_liquid_glass`), `apply_vibrancy` fallback on older macOS.
- **Database**: SQLite via `sqlx` (runtime-tokio). `registry.db` + one `workspace.db` per workspace. Two migration sets: `src-tauri/migrations/registry/` and `src-tauri/migrations/workspace/`.
- **Search**: SQLite FTS5; later `sqlite-vec`.
- **AI (keep it simple, like Meetily)**: `whisper-rs` (metal) compiled in, model `large-v3-turbo`; Qwen through Ollama at `http://localhost:11434` (`/api/chat`, streaming, `options.num_ctx` always set), model `qwen3.5:9b` with thinking ON. Exactly two capabilities: `transcribe()` and `complete()`. Jobs run one at a time. No cloud provider exists in this codebase.
- **Recorder**: `cpal` (mic) + `cidre` Core Audio process tap (system audio) + `rubato` mixer → incremental WAV. Transcription is a batch job after Stop — except a **Live** session (D-041): the mixer offers each 600 ms window to `kernel/ai/live.rs` over a bounded channel; it cuts utterances with the VAD, transcribes each in Vietnamese and translates it to English, writing lines as they arrive.
- **Bridge**: `tauri-specta` generates `src/bindings.ts` from Rust commands. UI calls commands only through `src/kernel/bridge/`.
- **Identity**: product name "New Dawn", bundle id `co.majorisdigital.newdawn`. Never change the bundle id.

## Workflow
THINK → ALIGN → SEE → BUILD. On every prompt: summarise the plan in ≤ 10 bullets and wait for "go" before writing code. Prefer SURGICAL changes; if you want to rewrite a working file, ask first.
Bridge between sessions: `LAST_BUILD.md` (what was built, manual steps) and `DECISIONS.md` (D-001… with rationale). Update both at the end of every build.

## Architecture rules (non-negotiable)
1. **Everything data goes through `kernel/hub.rs` (WorkspaceHub).** No module opens a database file. Cross-workspace views fan out across open pools in the hub.
2. **Everything AI goes through `kernel/ai/`.** No feature calls Ollama or Whisper directly. `ai/` owns the two models, a sequential job queue (one AI job at a time), streaming and `ai_log`.
3. **No network except `localhost`** — with one exception: `src-tauri/src/kernel/ai/models.rs` downloads model files from `huggingface.co` (redirects only to its `*.hf.co` CDN) at a pinned commit, only when the user clicks Download, SHA-256 verified (D-014). `scripts/check-localhost-only.sh` must pass: only `localhost`/`127.0.0.1` URLs in `src-tauri/src` and `src` (documentation links in comments and that one file excepted).
4. **The recorder never blocks the UI.** Audio callbacks push into bounded channels; everything else is async tasks.
5. **Thin commands.** `#[tauri::command]` functions in `commands/` validate input and call a module; they contain no logic.
6. **One error type** (`error.rs`, `AppError`), every command returns `Result<T, AppError>`; errors serialise to `{ code, message }`.
7. **Migrations are append-only**; the `migration-safety` agent reviews any migration before it runs.

## Design rules
- **Apple owns the shell and the controls; Majoris owns the palette and type (D-033).** Window background, sidebar and toolbar are native glass (window is `transparent: true`, body transparent); buttons, segmented controls, search and menus are capsules. The content area paints a near-solid surface over the glass (`--canvas` at ~92 % under normal transparency, 100 % under `prefers-reduced-transparency`). Colours and type come from the Majoris Design System: Slate neutrals, **orange as the one primary** (`--primary`, 500 fills / 600 hover / 700 pressed and text), green / yellow / red status.
- **The workspace brand is for documents only.** A tenant's `brand/tokens.json` (schema v2 in `.claude/skills/newdawn-ui-patterns`) feeds exported documents (minutes DOCX/PDF, later PPTX) and nothing in the UI except the logo in the workspace badge. No CSS variable is ever set from a pack. **Meeting types** (template + summary structure + email format) are the unit of per-tenant configuration.
- **Tint sparingly.** Orange on the primary button, the active sidebar item, chips, eyebrows, progress and links — never a wash over glass, never on secondary buttons.
- **System settings win**: dark mode via `prefers-color-scheme`; accent colour via `AccentColor` for selection/focus; `prefers-reduced-transparency` turns all CSS glass solid; `prefers-reduced-motion` disables transitions.
- **Golden Gate shape**: edge-to-edge sidebar, unified toolbar, a single corner radius for the window and major panels, overlay title bar with hidden title.
- **Transparency pattern (ghost sections)**: every dossier/detail section always renders its structure; empty fields show "Not yet captured", empty lists show 2–3 dashed ghost cards. Never hide a section because it's empty (type-conditional sections excepted).
- **Spacing rhythm** 8 / 12 / 16 / 24; hairlines 0.5 px; sizes and classes in the `newdawn-ui-patterns` skill.

## Code patterns
- Rust: modules under `src-tauri/src/{kernel,recorder,modules,commands}`; `anyhow` internally, `AppError` at the boundary; `tracing` for logs; `Arc<RwLock<_>>` for shared state, `AtomicBool` for flags; `tokio::sync::mpsc` **bounded** channels.
- TS: feature folders under `src/modules/<feature>/`; TanStack Router for pages; TanStack Query around bridge calls; local `useState` only — no global state libraries; URL state for filters/tabs.
- Naming: components PascalCase, helpers camelCase verb-first, constants SCREAMING_SNAKE, files match export names.
- No `any`. Use generated types from `src/bindings.ts`.

## Database conventions
- ids: text UUID v7; every table has `created_at`, `updated_at` (ISO-8601 text, UTC).
- JSON columns only for genuinely nested data; arrays as JSON text; FTS5 virtual tables kept in sync by triggers.
- `PRAGMA journal_mode=WAL; foreign_keys=ON;` on every pool. Checkpoint and close pools on exit.
- Adding schema: `sqlx migrate add <name> --source src-tauri/migrations/<registry|workspace>` → review with the `migration-safety` agent → migrations run automatically on open.

## Never do
- No cloud AI, no API keys, no telemetry, no network beyond localhost (model downloads in `kernel/ai/models.rs` excepted, rule 3).
- No Tailwind / CSS-in-JS / other fonts / global state libraries / `any`.
- No direct DB or AI calls outside the hub and gateway.
- No unbounded channels in the recorder. No blocking calls on the main thread (all `window-vibrancy` calls happen on the main thread via `app.run_on_main_thread`).
- No destructive migrations without explicit confirmation. No rewriting working files when a small change suffices.
- No "coming soon" empty cards — render the structure.

## After every build
1. `cargo check` (in `src-tauri`) and `pnpm tsc --noEmit` — clean.
2. `pnpm tauri dev` — open the screen you built; for UI, check light + dark and Reduce Transparency on.
3. Update `LAST_BUILD.md` (what changed, manual steps, env/config changes).
4. Add `DECISIONS.md` entries for any architectural choice.
5. Run the `architect-reviewer` agent on changed files; `migration-safety` if schema changed.
6. Run `/build-check`.

## Key files
- `src-tauri/src/lib.rs` — app setup, plugins, menu, glass, hub, gateway state (`main.rs` only calls `run()`)
- `src-tauri/src/kernel/registry.rs`, `hub.rs`, `theme.rs`, `glass.rs`, `menu.rs`, `tray.rs`, `permissions.rs`
- `src-tauri/src/kernel/ai/{mod,ollama,whisper,queue,log}.rs`, `src-tauri/src/kernel/brand/{tokens,logo}.rs`, `src-tauri/src/kernel/export/{model,docx,rebrand,pdf}.rs`, `src-tauri/src/kernel/profile.rs` (owner name + per-workspace title)
- `src-tauri/templates/minutes-base.docx` (the base every generated minutes template is made from; `scripts/make_minutes_base.py`)
- `src-tauri/src/recorder/{session,mic,system_tap,mixer,saver}.rs`, `src-tauri/src/modules/meetings/{transcribe,summarise,email}.rs`
- `src-tauri/src/modules/*.rs`, `src-tauri/src/commands/*.rs`
- `src/kernel/{shell,theme,glass,dossier,bridge}/`, `src/modules/{meetings,tenant-setup,recorder,models,workspace-settings}/`, `src/styles/{base,tokens}.css`
- `brands/<slug>/tokens.json` starter packs; `schemas/` dossier schemas
