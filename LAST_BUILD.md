# Last Build — P3.1 stage B (Summary tab)
_2026-10-24_

## P3.1 stage B
- **Summary tab** (`src/modules/meetings/SummaryTab.tsx`): "Ollama isn't running → Open Ollama" card (rechecks every 10 s), progress card ("Summarising · N tokens", part i/n, Cancel), email subject + model + "Edited" marker, **Copy as email** (HTML + text on the clipboard), **Open in Mail**, **Re-run** / **Deep re-run** (thinking on; a second click confirms replacing hand edits), every section rendered with an **Edit** → Markdown textarea → Save (re-renders the email), collapsible email preview; ghost sections + "Write summary" before a summary exists.
- Bridge `src/kernel/bridge/summary.ts`; `useSummarySync` in `AppLayout`; global `.srOnly`.
- Architect review: no must-fix (the Rust-sanitised `dangerouslySetInnerHTML` judged acceptable; block-level HTML, autolink and code-fence cases added to the escaping test). Applied: one `useSummarise` with its error shown; load errors no longer look like "no summary" (no accidental overwrite); an emptied section shows "Not yet captured"; Save locked while a summary is written and editors reset on a new summary; textarea autofocus, `role="alert"` errors, the replace-edits prompt announced, Deep re-run explained in visible text (`aria-describedby`); duplicate `.sectionTitle`/`.srOnly` removed; generated `SummaryProgress` type; progress cleared with `setQueryData(null)` (also for transcription); model name from Rust; failed "Open Ollama" shows a hint.
- Smoke launch: no runtime errors; **migration 0002 applied to the VIFC workspace** (`_sqlx_migrations` v2, tokenizer `remove_diacritics 2`).

## Manual steps — the P3.1 done-check
1. Make sure Ollama is running (`ollama list` shows `qwen3.5:9b`). `pnpm tauri dev`.
2. Record (or import) a real meeting → after the transcript, the summary starts by itself → "Summary ready" notification.
3. Summary tab: read it as if you were sending it; edit a section if needed; **Copy as email** → paste into Mail/Outlook/Gmail (formatting should survive); or **Open in Mail**.
4. Optional: **Deep re-run** on a hard meeting and compare (takes several minutes).
5. Quit Ollama and open the tab: the "Ollama isn't running" card appears; open Ollama → it clears within ~10 s.

---

# Vietnamese accent-insensitive search (migration 0002)
_2026-10-24_

- **User-approved** FTS rebuild: `src-tauri/migrations/workspace/0002_fts_remove_diacritics.sql` — `remove_diacritics 2` + a `folded` column (đ→d) for `transcript_fts` and `summary_fts`, refilled from the source tables, triggers recreated. D-022.
- Search: summary snippets from `email_md`; test now requires "tieng viet" and "dong y doi tac" to match.
- Verified in a scratch DB (rebuild after VACUUM, insert/update/delete/cascade, snippets keep the real spelling) and by the test suite (24 pass, migrations applied to temp workspaces). `migration-safety`: **SAFE** (rowid rebuild consistent, no collisions, triggers correct, one transaction; no REPLACE / no app writes to `fts_rowid`). `build.rs` now emits `rerun-if-changed=migrations` so new migration files are always embedded.
- Runs automatically on your workspaces at the next app launch (sqlx migrations on open).

---

# P3.1 stage A (Ollama gateway, summary, email)
_2026-10-24_

## P3.1 stage A
- `kernel/ai/ollama.rs` (status, unload, `TextSession`, streamed `complete`, `strip_think`); `ai_log` now records prompt/output tokens; Whisper ↔ Qwen hand-off (never both loaded).
- `modules/meetings/summarise.rs` (default structure EN/VI, house-rules prompt, prompt-shaped JSON + lenient parse + repair, chunk/map/recursive reduce, job with progress events + notification, view/edit/email parts/mailto) and `email.rs` (subject + body rendering, safe Markdown → HTML).
- Summary auto-runs after each transcript; launch recovery settles `summarising`.
- Commands: `get_summary`, `summarise_meeting(deep)`, `cancel_summary`, `update_summary_section`, `copy_summary_email`, `open_summary_in_mail`, `ollama_status`, `open_ollama_app`. Events `SummaryProgress`, `SummaryFinished`.
- Crates: `reqwest` + `json`, `pulldown-cmark` 0.13, `tauri-plugin-clipboard-manager` 2.
- Tests: 24 pass (new: think stripping, email EN/VI, HTML escaping, chunking, shaping, JSON parsing, mailto). Live (`--ignored`): `summarises_with_qwen` → full email in **24 s**, owners/dues/decisions/open question correct. D-021.
- Architect review: no must-fix. Applied: cancel honoured while queued/loading; `num_predict` cap per call (6k, 12k deep) + `output_truncated` on `done_reason: length`; Ollama clients use `.no_proxy()` (no environment proxy can carry meeting text off the Mac); email HTML keeps only the text of links/images; `<`/`>` in meeting text can't close the data tags; hand edits blocked while a summary is being written; overflow rows keep their token counts; unused `format` option removed. Live re-run: 16.6 s.
- No UI yet (stage B).

---

# P2.3 (post-meeting flow, search, notification)
_2026-10-24_

## P2.3
- Already from P2.1/P2.2: Stop → WAV finalised → `recorded` → transcription queued; one AI job at a time; retry via "Transcribe again"; per-tenant Meetings list with status chips.
- New: macOS notification when a transcript is ready/failed (`tauri-plugin-notification`); **search** on Meetings (`?q=`, titles + transcripts + summaries via FTS5, highlighted snippets, "match at mm:ss"); clicking a hit opens the meeting at that line (`?t=`); progress **card** on the meeting page.
- Rust: `meetings::fts_query`, `meetings::search`, `MeetingHit`, command `search_meetings`, `notify_finished`. Tests: 17 pass (new: FTS escaping, search over titles/transcripts/prefix/operators).
- UI: `Snippet.tsx`, `MeetingsPage` (search field), `TranscriptView` (`startMs`), `MeetingPage` (progress card). D-020.
- Architect review: no must-fix. Applied: transcript join on the segment's primary key (no scan of `fts_rowid`); every matching meeting listed even beyond the 500-row snippet limit; punctuation-only words ignored in queries; title search matches each word (any order); progress card no longer a chatty live region; per-meeting progress-bar labels; result count announced (`aria-live`), input in `<search>`; empty results keep ghost cards; summary-only hits open the Summary tab; dynamic title SQL marked with `sqlx::AssertSqlSafe` (only the count of a constant fragment varies).
- **Open question for you**: accent-insensitive Vietnamese search ("tieng" → "Tiếng") needs a migration that rebuilds the two FTS tables with `remove_diacritics 2` (derived data only, rebuilt in the same migration). CLAUDE.md requires your OK for DROP statements.

## Manual steps — the P2.3 done-check
1. `pnpm tauri dev` → ● Record → talk ~1 min (headphones) → ■ Stop.
2. Without touching anything: the meeting page shows the progress card, and a few minutes later the transcript appears and a "Transcript ready" notification shows (in dev it may be attributed to Terminal/VS Code).
3. Meetings → type a word you said → the result shows a highlighted snippet and "match at mm:ss"; click → the transcript scrolls to that line and the player is cued there.

---

# P2.2 stage B (recorder UI)
_2026-10-24_

## P2.2 stage B
- Toolbar **record pill** (live), **Start dialog** (⌘R, File → New Recording, tray "Record / Stop"), **silence banner**, **Settings → General → Permissions**. Stop opens the meeting page with transcription queued. D-019.
- Files: `src/kernel/bridge/recorder.ts`, `src/modules/recorder/*`, `AppLayout`, `Toolbar`, `GeneralTab`; shared sheet styles moved to `components.css`.
- Architect review: 1 must-fix — stopping after a mid-call workspace switch (or closing the start dialog while it was starting) opened a meeting page that couldn't find the meeting; Stop now switches to the recording's workspace first. Also applied: start/import sheets can't be dismissed while pending; meeting type resets when the workspace changes (shared `MeetingFields` replaces ~40 duplicated lines); stop/pause errors shown on the pill; dark-mode `--status-*` colours ≥ 4.5:1; banner is an always-mounted live region (`alert` when silent); `openPrivacySettings` never rejects; "them" meter neutral (toolbar is outside the tenant brand). Not done: folding `Settings.module.css` into the global section classes (no behaviour change).
- Checks: clippy, 15 tests, tsc, build, localhost check clean; app launched by Claude with no runtime errors (UI not visually verified).

## Manual steps — the P2.2 done-check
1. `pnpm tauri dev`. Put on **headphones**.
2. Start a Zoom or Google Meet test call (zoom.us/test, or a Meet with a second device).
3. **● Record** (or ⌘R) → workspace, title, English → Start. Allow microphone / system audio if macOS asks (in dev they're attributed to VS Code / Terminal).
4. Talk and let the other side talk: both meters should move (red = you, brand = them). If the banner says it can't hear the other side, follow its button, then start a new recording.
5. Pause for ~10 s, resume, then ■ Stop → the meeting page opens and transcription runs; the player should contain both sides with the pause cut out.
6. Crash test: start another recording, talk for ~20 s, then kill the app (`pkill -9 new-dawn` or force-quit). Relaunch → the meeting shows as recorded and the audio plays (at most ~2 s lost).
7. The WAV is at `~/Library/Application Support/co.majorisdigital.newdawn/workspaces/<slug>/recordings/<id>/audio.wav`.

---

# P2.2 stage A (recorder backend)
_2026-10-24_

## P2.2 stage A
- `src-tauri/src/recorder/`: `mic.rs` (cpal), `system_tap.rs` (cidre process tap + private aggregate device), `mixer.rs` (resample, soft limit, high-pass, loudness, alignment, mix, levels, silent-tap detection), `saver.rs` (crash-safe WAV + `repair`), `session.rs` (state machine, capture/mixer threads, 10 Hz meter).
- `kernel/permissions.rs` (microphone status/request, System Settings URLs). Commands: `recorder_status`, `start_recording`, `set_recording_paused`, `stop_recording`, `microphone_permission`, `request_microphone_permission`, `open_privacy_settings`. Events: `RecorderStatusChanged`, `RecorderLevel`, `MenuAction::ToggleRecording`.
- File → New Recording (⌘R) and the tray "Record / Stop" item are enabled (UI handles them in stage B).
- Meetings: `create` takes a status; `delete`, `finish_recording`, `recover_recordings` (run at launch before `recover_interrupted`).
- Crates: `cpal` 0.18, `ebur128` 0.1, `cidre` 0.29 (core_audio, av, async, dispatch, macos_14_2).
- Tests: 15 pass (new: WAV write/repair, mix scaling/backlog, high-pass, resampler). Live (`--ignored`): `live_capture` (3 s mic + tap while `say` plays: both sides, 0 drops) and `mic_raw_levels` — both run on this Mac. Mic permission was granted to the launching app (VS Code) during the first run.
- Architect review: 2 must-fix, both fixed — (1) two overlapping starts could both record: `start` now claims a `starting` flag for its whole duration; (2) `stop` could leave the session stuck in Stopping: it now always clears the session, with a 10 s join timeout and a repair fallback (`failed` if unrepairable). Also applied: tap format/buffer checks; `SystemTap` frees its context via `Box::into_raw`/`Drop` after the device stops; mic stall pads silence so the other side keeps recording; launch recovery moved to `recorder::session::recover` (meetings stays pure data), 0-sample recordings → `failed`, one bad workspace no longer blocks launch; blocking work moved off async paths; ponytail note on the per-chunk allocation in the IO proc.
- D-018. No UI yet (stage B).

---

# P2.1 stage D (Core ML measurement)
_2026-10-24_

## P2.1 stage D
- `cargo clean` (user-approved; 21.4 GB of stale build output removed; disk went from 6.4 GB to 21 GB free).
- Added an opt-in `coreml` cargo feature (off) and the `bench_transcribe` test (`NEWDAWN_BENCH_MODEL`, `NEWDAWN_BENCH_AUDIO`).
- Measured Metal vs Core ML on 10 min with large-v3: ~63.7 s vs ~62.5 s median (Core ML first run 146 s, one 116 s outlier) → **stay Metal only** (D-017). The downloaded encoder (1.2 GB, SHA-256 verified) was in a scratch folder and has been deleted.
- Manual: none. Next: your 30-minute Voice Memo import (stage C steps below), then P2.2.

---

# P2.1 stage C (import + meeting UI)
_2026-10-24_

## P2.1 stage C
- File → **Import Audio…** (⌘⇧I) and Meetings → **Import audio…** open the import sheet (native picker via `tauri-plugin-dialog`); creates the meeting and starts the job, then opens the meeting page.
- **Meetings** lists the current workspace's meetings (date, duration, status chip, live progress bar).
- **Meeting page** `/meetings/$id`: title, date · duration · language · status; Cancel while transcribing; Transcribe / Transcribe again; tabs Transcript (sticky player, timestamped rows, click-to-seek, playing row highlighted) | Summary | Minutes (ghost structure).
- **Settings → Models → Memory**: Whisper idle unload 1 / 2 / 5 / 15 min (saved in `registry.db`, applied live).
- Rust: `list_meeting_types`, `get/set_whisper_idle_unload`, `hub.app_setting`, menu item enabled; crates `tauri-plugin-dialog` (+ npm `@tauri-apps/plugin-dialog`); capability `dialog:allow-open`. D-016.

## Architect review
One must-fix: importing into a non-current workspace opened "Meeting not found" — the sheet now switches to that workspace first. Applied: workspace picker no longer starts empty on a cold open; the sheet is a native `<dialog>` (focus trap, Esc); closing it no longer adds a Back entry; asset scope narrowed to `workspaces/*/recordings/**`; idle-unload logic moved out of the command; aria labels (language group, memory group, timestamps, player, progress); error lines instead of silent ghosts; "Not yet captured" on Summary/Minutes; 0.5 px dashed border. Skill updated (modal sheets may use `.glass`).
**Resolved**: `localhost:11434` removed from the webview CSP `connect-src` (user decision, D-016).

## Manual steps (the P2.1 done-check)
1. Your running `pnpm tauri dev` has already rebuilt. File → Import Audio… → choose a **30-minute Voice Memo** (`~/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings/*.m4a`, or export one from Voice Memos) → title → English → Import and transcribe.
2. Watch the progress bar (stages: reading → finding speech → transcribing %). Try Cancel once, then Transcribe again.
3. Read the transcript; click timestamps to seek. Check `ai_log` and that Whisper unloads after the idle period (log line `whisper model … unloaded`).
4. Light/dark, Reduce Transparency (the sheet goes solid).

---

# P2.1 stage B (transcription engine)
_2026-10-24_

## P2.1 stage B
- `kernel/ai/whisper.rs` — `WhisperEngine` (load on use, idle unload, VAD → Whisper → cleanup), `Language`, `clean_text` (+ test); raw abort callback works around a whisper-rs 0.16 bug (D-015).
- `kernel/ai/queue.rs` (`AiQueue`), `kernel/ai/log.rs` (`ai_log` rows), `models.rs` (`preferred_speech_model`).
- `modules/meetings/{mod.rs (Meeting, status, list/get/create/transcript/save_transcript), audio.rs (decode → 16 kHz mono; test with `say`), transcribe.rs (import + background job + events; e2e test)}`.
- Commands: `list_meetings`, `get_meeting`, `get_transcript`, `import_audio`, `transcribe_meeting`, `cancel_transcription`. Events: `TranscriptionProgress`, `TranscriptionFinished`. `hub.workspace(slug)`.
- Crates: `whisper-rs` 0.16 (metal, tracing_backend), `symphonia` 0.6 (+aac, isomp4, mp3, aiff, alac, caf), `rubato` 5. Dev profile optimises dependencies.
- Architect review: one must-fix (early exits after `transcribing` skipped status + `ai_log`) — fixed by moving queue/blocking/`ai_log` into the `kernel::ai::whisper::transcribe` gateway and giving the job a single exit path. Also applied: re-run guard, previous status restored on cancel, launch recovery of interrupted jobs, cancel-while-queued, poison-tolerant locks, fsynced `transcript.json` after the DB save, ponytail note on in-memory audio. `greet` kept (P0).
- Tests: 9 pass; `cargo test -- --ignored` adds the network download test and `transcribes_generated_speech` (real large-v3: word-perfect, 4.7 s; mid-job cancel; FTS; launch recovery) — both pass.
- No UI yet (stage C). Manual: none.

---

# P2.1 stage A (model downloads)
_2026-10-24_

## P2.1 stage A
- `src-tauri/src/kernel/ai/models.rs`: catalogue (turbo, turbo-q5_0, large-v3, Silero VAD v5.1.2) pinned to HF commits with exact sizes + SHA-256; `ModelStore` (list, begin/finish/cancel, delete, download with Range resume, 30 s stall timeout, SHA-256 verify, atomic rename). Tests: `verify_checks_size_and_hash`; `download_resume_and_verify` (network, `#[ignore]`, run once: passed).
- `commands/models.rs`: `list_models`, `start_model_download` (background task), `cancel_model_download`, `delete_model`, `reveal_models_folder`; events `ModelDownloadProgress`, `ModelDownloadFinished`.
- Settings → Models is real: `src/modules/models/ModelsPanel.tsx`, `src/kernel/bridge/models.ts`.
- Crates: `tokio` (fs, io-util, time, sync), `reqwest` 0.13 (rustls, http2, stream), `futures-util`, `sha2`.
- **CLAUDE.md rule 3 amended** (approved): huggingface.co downloads allowed only in `kernel/ai/models.rs`; `scripts/check-localhost-only.sh` allows exactly that. D-014.
- **Silero VAD is bundled** (follow-up): `src-tauri/resources/models/ggml-silero-v5.1.2.bin` (+ README with source/hash/licence), `tauri.conf.json` → `bundle.resources`; removed from the download catalogue; Settings → Models shows it as "Built in". Test `bundled_vad_matches_pin`.
- State on this Mac: `large-v3` downloaded and verified (3.1 GB); turbo not downloaded; ~11 GB free. The earlier downloaded `models/ggml-silero-v5.1.2.bin` in app data is now unused and can be deleted.
- Manual: Settings → Models shows three Whisper models + "Silero VAD · Built in".

---

# P1.4 stage A (wizard mockup) on top of P1.3
_2026-10-24_

## P1.4 stage A — SEE
- `/setup` wizard mockup (static data): Identity → Brand → Meeting types → Review, step in the URL (`?step=`), sticky Back / Continue footer. Files: `src/modules/tenant-setup/*`.
- Workspace → **New Workspace…** and **Tenant Setup…** now open it (`MenuAction::OpenTenantSetup`); the switcher's "New workspace…" row too.
- New global `src/styles/components.css`: page/section typography, `.input`, `.btn` (+ `btnPrimary/Secondary/Ghost/Compact`, shape from `--button-shape`), `.segmented`, `.chip`/`.chipBrand`, `.card`.
- Manual: `pnpm tauri dev` → Workspace → Tenant Setup… → click through the four steps (light + dark). Nothing is saved yet.

---

# P1.3 Sidebar, toolbar, workspace switcher, menu bar, tray, settings window
_2026-10-24_

## Summary
Real sidebar driven by the hub (workspace badge + switcher popover, Meetings, inert Record, Settings), current workspace persisted in `registry.db`, native menu bar, menu-bar tray item, and a separate Settings window with tabs General / Workspaces / Models / Converters / About. The main window now hides on close (Quit exits).

## Files created / modified
- Rust
  - `src/kernel/menu.rs` (menu bar + menu/tray event handler), `kernel/tray.rs`, `kernel/windows.rs` (Settings window, show main), `events.rs` (typed events), `commands/window.rs` (`open_settings`)
  - `kernel/hub.rs` (`current_workspace`, `set_current_workspace`; new test `current_workspace_falls_back`), `kernel/registry.rs` (`setting`, `set_setting`, `touch_opened`), `commands/workspace.rs` (`get/set_current_workspace`; mutations emit `WorkspacesChanged`)
  - `lib.rs` (menu, window-state plugin, hide-on-close, Reopen, tray, events), `Cargo.toml` (`tauri-plugin-window-state` 2, tauri `image-png`, tauri-specta `derive`), `capabilities/default.json` (main + settings), `icons/tray.png` (generated)
- UI
  - `kernel/shell/{WorkspaceSwitcher,useWorkspaceShortcuts}`; `Sidebar`, `AppLayout`, `Toolbar` updated (sidebar collapse, toolbar clears the traffic lights)
  - `kernel/bridge/{events,settings,app}.ts`, `workspaces.ts` (`useCurrentWorkspace`, `useSetCurrentWorkspace`)
  - `modules/workspace-settings/{SettingsPage,GeneralTab,AboutTab,GhostTab,WorkspacesPanel}` + CSS; `modules/meetings/MeetingsPage` (ghost structure); `router.tsx` (shell layout vs settings; `/` → `/meetings`; dev route only in dev builds)
  - Removed: `/dev/workspaces` page (now Settings → Workspaces), `Placeholder.tsx`
- `src/bindings.ts` regenerated (commands + events)

## Build state
- `cargo check`, `clippy --all-targets -D warnings`: clean; `cargo test`: 4 passed
- `pnpm typecheck`, `pnpm build`, `pnpm check:localhost`: clean
- `pnpm tauri dev`: launched for 150 s by Claude with no panic; visuals and menu items not verified by Claude

## Manual steps required
1. `pnpm tauri dev`. Sidebar: badge shows VIFC; click it → popover; pick another workspace (create one in Settings → Workspaces first); ⌘1/⌘2 switch; relaunch → the choice persists.
2. Menu bar: New Dawn → Settings… (⌘,); File: New Recording and Import Audio are greyed out; View → Toggle Sidebar (⌘⌥S); Workspace → Switch… opens the popover, New Workspace… opens Settings → Workspaces, Tenant Setup… is greyed; Help → Reveal Data Folder in Finder.
3. Menu-bar sunrise icon: Record (greyed), Open New Dawn, Quit New Dawn. Close the main window with the red button → app stays in the menu bar; click the Dock icon or "Open New Dawn" to bring it back.
4. Settings: move the window, quit, reopen → same position. Create/archive a workspace there → the main window's badge and popover update live.
5. Light, dark and Reduce Transparency on: sidebar labels, popover, Settings tabs.

## Architect review
No must-fix. Applied: stable event subscription in `useWorkspacesSync`, Toolbar search/Record disabled, popover no longer claims `role="menu"`, readable glass label. Kept per spec: `greet` (P0), the skill's 12 px section titles and brand-100 active rows.

## Open items
- **P1.4**: the sidebar and switcher badge sit outside `.content-root`; `applyBrand()` must set brand variables on a wrapper that contains the sidebar too (or the layout root), otherwise they stay default blue.
- Check the Settings window in a release build (`pnpm tauri build`): it opens `settings?tab=…`, which relies on the bundle falling back to `index.html`; dev mode works.
- `pnpm tauri dev` warns that `@tauri-apps/api` 2.12.4 and the `tauri` crate 2.11.6 differ in minor version. It runs, but aligning them (`cargo update -p tauri`) is worth a separate, small change.
- The tray icon is a placeholder template image; the real icon lands in P4.1.
