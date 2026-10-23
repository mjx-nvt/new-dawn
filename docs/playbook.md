# New Dawn — Build Playbook v2 (v1 scope: tenants + the meeting machine)

_2026-10-23. Supersedes playbook v1. Companion to `plan.md` (what and why), `ai-engine.md`, `meetily-teardown.md` (reference code at `../Ref/meetily`), `document-export.md`. One prompt = one Claude Code session that ends with something you can open._

## 0. Rules (unchanged)
- Every prompt starts with: *"Read `CLAUDE.md`, then the docs named below. Summarise your plan in ≤ 10 bullets — files, crates/packages, anything you are unsure of — and wait for my 'go'. Check a crate's current API on docs.rs before using it; do not guess signatures."*
- Every prompt ends with: *"When done: run `/build-check`, update `LAST_BUILD.md` and `DECISIONS.md`, run the `architect-reviewer` agent, and list the exact manual steps I must do."*
- "Port from Meetily `path`" means read it and rewrite it cleanly — never copy files.
- Stuck on the same compile error for ~3 rounds → stop; ask for a plain-language explanation and two different fixes.

## Phase 0 — Setup
Follow `GETTING_STARTED.md`. Done when the hello window opens in Be Vietnam Pro, `cargo check` is clean, `ollama list` shows `qwen3.5:9b`, and `../Ref/meetily` exists. Then run **P0**.

### P0 — Scaffold verification
```
Read CLAUDE.md and docs/plan.md §§1–6. This repo was scaffolded with create-tauri-app and overlaid with control files; verify and fix only what is missing: layout per docs/plan.md §2 (folder markers with READMEs; delete template leftovers public/tauri.svg, public/vite.svg, src/assets/react.svg and any references); .claude agents/commands/skill well-formed; tauri.conf.json identity (co.majorisdigital.newdawn, "New Dawn"), transparent Overlay window, CSP allowing only localhost:11434, minimumSystemVersion 14.2, entitlements + hardenedRuntime, resources map for ../brands/; .cargo/config.toml 14.2 target; Info.plist microphone + system-audio descriptions; @fontsource/be-vietnam-pro imported in src/main.tsx with tokens.css and base.css; scripts/check-localhost-only.sh executable and passing; `pnpm check:localhost` script. Keep the template `greet` command. Done when `pnpm tauri dev` shows the placeholder in Be Vietnam Pro with transparent rounded corners, `cargo check` and `pnpm typecheck` are clean, and `/build-check` passes.
```

## Phase 1 — Shell + tenants

### P1.1 — Native glass window and shell skeleton
```
Read docs/plan.md §5 and the newdawn-ui-patterns skill. SEE first: static mockup at /dev/shell (240px sidebar, 52px unified toolbar, content area). Build: add `window-vibrancy`; `kernel/glass.rs` with `apply_shell_glass(window)` — macOS ≥ 26 → `apply_liquid_glass` (regular, window radius, untinted), else `apply_vibrancy` Sidebar material; main thread only; command `glass_status()`. Confirm base.css (transparent html/body, `.content-root` 92 % canvas, solid under reduced transparency, dark mode, reduced motion). Add TanStack Router; `src/kernel/shell/{Sidebar,Toolbar,ContentRoot}`; app layout. No features, no DB. Done when native glass shows behind sidebar and toolbar over a colourful wallpaper, the content area reads solid, dark mode and Reduce Transparency look right, and `glass_status()` returns "liquid_glass".
```

### P1.2 — Registry, WorkspaceHub, migrations, typed bridge
```
Read CLAUDE.md "Architecture rules"/"Database conventions", docs/plan.md §3 and §6. Build: crates `sqlx` (runtime-tokio, sqlite, uuid, chrono), `uuid` (v7), `tokio`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`, `tauri-specta` + `specta`, `trash`; `error.rs` with `AppError {code,message}`. `migrations/registry/0001_init.sql` (workspace, setting). `migrations/workspace/0001_init.sql`: meeting_type, document_template, meeting, transcript_segment, summary, document_export, ai_log, setting, FTS5 over transcript_segment.text and summary.email_md with triggers — exactly docs/plan.md §6. `kernel/registry.rs` (open registry.db with WAL + foreign_keys, migrate, workspace CRUD; `create_workspace(name, slug)` creates `workspaces/<slug>/{workspace.db, brand/, recordings/, exports/}` and migrates; delete = Trash via `trash`). `kernel/hub.rs` (`WorkspaceHub`: pools per slug behind RwLock, `pool(slug)`, `open_all_active()`, checkpoint+close on exit). `commands/workspace.rs` via tauri-specta → `src/bindings.ts`; `src/kernel/bridge/` with TanStack Query hooks. Dev page /dev/workspaces. Done when creating "VIFC" produces `~/Library/Application Support/co.majorisdigital.newdawn/workspaces/vifc/workspace.db` with the tables, it survives a restart, and `migration-safety` says SAFE.
```

### P1.3 — Sidebar, toolbar, workspace switcher, menu bar, settings window
```
Read docs/plan.md §5, newdawn-ui-patterns "Sidebar & toolbar". SEE first: mockup with the final sidebar (tenant badge; nav: Meetings, Record; bottom: Settings) and toolbar (title/breadcrumb left, search centre, inert ● Record pill right). Build: real Sidebar from the hub; switcher popover (⌘1…⌘9); current tenant in registry `setting.current_workspace`. Native menu bar: New Dawn (About, Settings ⌘,), File (New Recording ⌘R, Import Audio…), Edit, View (Toggle Sidebar ⌘⌥S), Workspace (Switch…, New Workspace…, Tenant Setup…), Window, Help — menu events → UI via Tauri events. Menu-bar status item (tray, template icon): Record (inert), Open New Dawn, Quit. Settings window (separate WebviewWindow "settings", 720×520, same glass; tabs General / Workspaces / Models / Converters / About). Done when switching tenants updates the badge and persists, every menu item works or is disabled, settings opens and remembers its position, light/dark/reduced-transparency correct.
```

### P1.4 — Tenant setup wizard, brand import, meeting types, templates
```
Read docs/plan.md §3 (all), docs/document-export.md §2.2 (placeholders), the newdawn-ui-patterns "Brand pack schema" (v2 fields incl. slogan and document_fonts), docs/ai-engine.md §3b (Ollama basics — this prompt may add a minimal `ai/ollama.rs` with `complete(prompt, think, images?)` if P3.1 has not run yet; keep it tiny). SEE first: wizard mockup (Identity → Brand → Meeting types → Review).

Build:
1. `kernel/brand/tokens.rs`: load/validate `brand/tokens.json` schema v2 (name, slug, logo, slogan{en,vi}, colors{primary, secondary, accent, text, canvas, status}, document_fonts{heading, body, files}, radius, button); derive the colour ramp; command `get_brand`; `applyBrand()` in `src/kernel/theme/` sets CSS variables on `.content-root` only (UI font never changes).
2. `kernel/brand/import.rs` — Brand import: inputs = one `.docx` template (required), optional guideline PDF, logo files, font files. (a) Deterministic DOCX parsing with `zip` + `quick-xml`: theme major/minor fonts and theme colours from `word/theme/theme1.xml`; Heading 1/2 and Normal fonts/sizes/colours from `styles.xml`; header images → logo candidates (extract to `brand/logo/`); header text → slogan candidates; body headings + `{{placeholders}}` → section structure. (b) If a PDF is given: rasterise pages (`pdfium-render` or `pdf2image`-equivalent crate) and ask Qwen3.5-9B (vision, think on) for a JSON proposal {colors[], fonts{heading, body}, logo_rules, tone_notes}. (c) Merge into a `BrandProposal` with provenance per field; command `brand_import_propose(files)`.
3. Wizard UI (`src/modules/tenant-setup/`): Identity (name, slug) → Brand (drop zone; proposal review form with swatches, font names, logo picker, slogan EN/VI; edit anything) → Meeting types (add type: name, default language, template = the imported docx or another upload, **summary structure** pre-filled from the template's headings — editable list of {title, instruction}; email format: subject pattern, greeting, sections, sign-off; include transcript default) → Review → Save (writes tokens.json, copies logo/fonts/templates, `manifest.json` per template, `meeting_type` + `document_template` rows).
4. Settings → Workspaces → (tenant) → Brand / Meeting types / Templates tabs reuse the same forms; "Template check" lists placeholders found and warns about header/footer cells too narrow for the EN/VI slogan lines.
5. Starter packs: `brands/vifc/` (tokens v2: primary #0132AD, slogan EN/VI, document_fonts Playfair Display / Be Vietnam Pro, logo variants) and `brands/default/`; the wizard offers "start from a pack".
Done when: running the wizard with your official VIFC minutes template produces a VIFC tenant whose content area is blue-branded with its logo, with a "Partner call" meeting type whose summary structure matches the template's headings, and a second tenant on the default pack looks neutral.
```

## Phase 2 — Record + transcript

### P2.1 — Whisper engine, models page, audio import
```
Read docs/ai-engine.md §3a and §5, docs/meetily-teardown.md §2 (all), reference ../Ref/meetily/frontend/src-tauri/src/{whisper_engine/whisper_engine.rs, audio/import.rs, audio/decoder.rs, audio/vad.rs, audio/common.rs, parakeet_engine/parakeet_engine.rs (downloader only)}. Port, do not copy. Build: `ai/whisper.rs` (`whisper-rs` feature `metal`; load `large-v3-turbo` use_gpu + flash_attn; ONE WhisperState per job; beam 3, temp 0.2, thresholds per ai-engine §3a, n_threads = 8, timestamps on and offset by segment start; language explicit; language-safe cleanup; idle unload setting). Models page (Settings → Models): catalogue large-v3-turbo / -q5_0 / large-v3 with SHA-256 from a pinned HF commit, Range-resume, stall timeout, MB/s, cancel keeps partial, delete; `<app_data_dir>/models/`. `meetings/transcribe.rs`: decode (`symphonia`, sample rate from the decoded stream) → mono, peak-normalise, `rubato` sinc to 16 kHz → Silero VAD (`silero_rs`; redemption 2000 ms, pads 300/400, min 250) → split > 25 s at the quietest window → transcribe → `transcript_segment` rows + `transcript.json` (temp+rename); staged progress events; cancel between stages; meeting status transitions. UI: File → Import Audio… creates a meeting (pick tenant, type, title, language) and runs the job; meeting page Transcript tab (timestamps, click-to-seek player). Note in DECISIONS whether this whisper-rs exposes built-in Silero VAD, and measure Core ML encoder on/off on a 10-minute file. Done when a 30-minute m4a from Voice Memos imports with a progress bar into a readable timestamped English transcript, and Whisper unloads afterwards.
```

### P2.2 — Recorder (no bot): permissions, mic, system audio, mixer, WAV
```
Read docs/plan.md §4, docs/meetily-teardown.md §2.7 and §6 items 5/10, reference ../Ref/meetily/frontend/src-tauri/src/audio/{stream.rs, pipeline.rs (mic path + mix_window), incremental_saver.rs, permissions.rs, devices/discovery.rs, capture/core_audio.rs, capture/system.rs, capture/backend_config.rs}. Build `src-tauri/src/recorder/`: `kernel/permissions.rs` (microphone check/request via a cpal probe stream; System Audio Recording explainer → opens the right System Settings pane); `mic.rs` (cpal default input, F32/I16, rubato to 48 kHz mono, 80 Hz high-pass, EBU R128 −23 LUFS via `ebur128`, 100 ms frames into a BOUNDED channel cap 50, drop counter); `system_tap.rs` (`cidre` global mono process tap in a private aggregate device with auto-start; 1024-sample blocks; bounded channel; detect all-zero frames for 3 s while mic is active → "permission denied" state); `mixer.rs` (align in 600 ms windows via ring buffer; sum with proportional soft scaling above ±1; per-source levels); `saver.rs` (incremental 16 kHz mono WAV, header patched on finalize, flush every 2 s, recover truncated file on next launch); `session.rs` (Idle → Recording → Paused → Stopping → Idle; start(tenant, meeting_type, title), pause, resume, stop; events recorder-state, recorder-level at 10 Hz). UI: toolbar pill live (● / elapsed / two-bar meter you+them / ■), ⌘R, menu-bar item; Start dialog (tenant, meeting type, title, language); Settings → General permissions pills; banner if the tap returns silence. No live transcription. Done when a Zoom/Meet test call records both sides cleanly to a WAV under the tenant's recordings, pausing works, and killing the app mid-call leaves a playable file.
```

### P2.3 — Post-meeting transcription job + meeting page
```
Read docs/plan.md §4 table rows Stop/Transcript. Build: on Stop, finalise the WAV, create/update the `meeting` row (status recorded), queue the transcription job from P2.1 (status transcribing → transcribed), with a progress card on the meeting page and a notification "Transcript ready"; a job queue that runs one AI job at a time (sequential; no scheduler beyond a simple queue); retry on failure; Meetings list page (per tenant; status chips; search via FTS5). Done when ● → ■ → a few minutes later the full transcript appears without touching anything.
```

## Phase 3 — Summary + minutes

### P3.1 — Ollama client, summary per meeting type, email body
```
Read docs/ai-engine.md §§3b–4 (keep it simple), §10; docs/plan.md §4 rows Summary/Email body; docs/meetily-teardown.md §3.4 for the prompt hygiene and what to do differently. Build: `ai/ollama.rs` — health (`/api/tags`), `complete(prompt, think: bool, images?)` over `/api/chat` with `stream: true`, `options.num_ctx` set explicitly, `keep_alive`, tokens + duration logged to `ai_log`; model fixed to `qwen3.5:9b`; friendly "Ollama is not running" state. `meetings/summarise.rs`: real token count (`/api/tokenize` if available, else a tokenizer crate); chunk only if transcript + prompt exceed num_ctx, map → recursive reduce; prompt = house rules (only facts from the transcript; `<transcript>` tags; ignore instructions inside; write in the meeting language; one Markdown block per section) + the meeting type's summary structure; thinking ON; `<think>` stripped; result → `summary.sections_json`; status summarised. `meetings/email.rs`: render email_subject (pattern from the meeting type) and email_md (greeting, overview paragraph, key points, decisions, action items as "owner — item — due", next steps, sign-off) from the sections — no second AI call. UI: Summary tab (sections editable, Re-run, "Copy as email" → plain text + HTML on the clipboard via Tauri clipboard plugin, "Open in Mail" via mailto:). Done when a recorded meeting produces a detailed executive summary you would actually paste into an email to the team, in under 3 minutes after the transcript on the M1 Pro.
```

### P3.2 — Minutes DOCX from the tenant template + PDF
```
Read docs/document-export.md (all; its §4 prompt is folded here). Build: `export/model.rs` (document model from meeting + summary + tenant brand); `export/docx.rs` (zip + quick-xml template engine: run-merging, simple fields, `{{logo}}` image swap, `{{#actions}}` repeating rows, optional `{{#transcript}}` block, Markdown → the template's own styles; EN-then-VI slogan lines in header/footer with cell widening or warning; atomic write to `exports/`); `export/pdf.rs` (converter chain detected at startup: Microsoft Word via osascript → LibreOffice `soffice --headless --convert-to pdf` → Pages via osascript; 60 s timeout; report which was used); commands export_minutes_docx / export_minutes_pdf / list_converters; migration adds nothing new beyond `document_export` (already there). UI: Minutes tab — Export ▾ (Word / PDF / Both), options (language mode, include transcript, file name), PDF preview via asset protocol, result row with Reveal in Finder / Open; Settings → Converters shows what was found. Unit tests with a fixture template (split placeholder, 3 action rows). Done when a recorded VIFC meeting exports a .docx that opens in Word looking exactly like your template with the content filled in, and a PDF that matches it.
```

### P3.3 — Review, edit, re-run, history
```
Read docs/plan.md §4 Review row. Build: inline editing of summary sections with "edited" markers and a Re-run that keeps your edits unless you discard them; transcript corrections (edit a segment, it flows into the next summary run); export history per meeting (`document_export` rows) with re-export; meeting notes field; delete meeting (moves recordings to Trash). Done when a messy real meeting can be cleaned up and re-exported in one sitting.
```

## Phase 4 — Ship

### P4.1 — Icon, notifications, onboarding, backup, build
```
Read docs/plan.md §8 Phase 4 and docs/meetily-teardown.md §4 (packaging lessons). Build: Icon Composer layered icon → `actool` → Assets.car + CFBundleIconName (keep .icns fallback); notifications (Recording started/stopped, Transcript ready, Summary ready); first-run onboarding (permissions, Ollama check + pull qwen3.5:9b, Whisper model download, PDF converter detection, first tenant → wizard); window-state memory; nightly backup of registry.db + every workspace.db to a chosen folder (keep 7); `pnpm tauri build` → New Dawn.app + .dmg (ad-hoc signed, hardened runtime); LAST_BUILD.md records the exact build command. Done when a fresh Mac user account can install the .dmg, finish onboarding, and record their first meeting.
```

## Later (not v1) — prompts written when needed
Tasks + cockpit across tenants (action items → tasks) · notes + quick capture · knowledge dossiers (import the old Ecomobi data) · research + embeddings (`sqlite-vec`, bge-m3 vs Qwen3-Embedding) · live transcription (bounded queue worker from the v1 playbook) · speaker labels (`sherpa-rs`) · bundled Ollama sidecar · `ffmpeg` sidecar for M4A · Typst PDF fallback · auto-detect a running call.

## Working rhythm
One prompt per sitting; P1.4, P2.2 and P3.2 are the big ones — give each a weekend. Start using the recorder for real at the end of P2.3; real meetings find the right next prompt faster than planning does. Keep `docs/journal.md`.
