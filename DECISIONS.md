# Architectural Decisions

Newest first. Every entry: what was decided, why, and what it means for future work. Keep entries short; link to `docs/` for the long version.

## D-026 — Ship: icon, onboarding, notifications, backup, logs, build
_2026-10-24_

**Decision** (P4.1, user-approved plan):
- **Icon**: a first-version sunrise (sky gradient, sun, horizon) as an Icon Composer file `src-tauri/icons/AppIcon.icon` → `actool` → `Assets.car` (Liquid Glass on macOS 26+, `CFBundleIconName` = AppIcon) plus a flat twin `AppIcon-flat.svg` → `icon.icns`/PNGs for macOS 14–15 (actool's own .icns stops at 256 px). `scripts/build-icon.sh` rebuilds both; the generated files are committed so a build needs no Xcode icon step.
- **Onboarding** `/welcome` in the main window until `onboarding_done` is set (index route redirects): Permissions (mic request, call-audio explained, test notification to trigger the macOS prompt) → Ollama (status, Open Ollama, **Download qwen3.5:9b** via the gateway's localhost `/api/pull`, progress in MB because the model is > 4 GB) → Whisper (Settings' model list) → PDF converters → Workspace (Settings' workspace panel) → Done. Every step skippable; New Dawn → Run Setup… shows it again. Existing installs see it once.
- **Notifications**: one helper `kernel/notify.rs`; Recording started / stopped (with length) added to Transcript ready / Summary ready.
- **Backup** `kernel/backup.rs`: off until a folder is chosen; `VACUUM INTO` copies of registry.db and every open workspace.db (archived = file copy) plus each workspace's `brand/` folder (and an archived workspace's WAL) into `<folder>/New Dawn <yyyy-mm-dd>/`, one at a time, built in a hidden partial folder then swapped in (leftovers pruned); same-day re-runs replace; prunes to 7, touching only folders named `New Dawn YYYY-MM-DD` that hold a registry.db. "Nightly" = due when the last backup is ≥ 24 h old, checked a minute after launch and hourly (no launchd agent). Recordings and exports are not included.
- **Privacy notes**: logs never carry meeting text (a malformed-JSON error keeps only its position); notification bodies carry the meeting title, which can show on the lock screen (macOS setting).
- **Logs**: `tracing-appender` daily files in `~/Library/Logs/co.majorisdigital.newdawn/`, 7 kept (Meetily shipped without logs — teardown §4).
- **Build**: `pnpm tauri build` → `New Dawn.app` + `.dmg`, ad-hoc signed (`signingIdentity: "-"`), hardened runtime, entitlements mic + Apple Events. No notarisation (needs a paid Developer ID): on another Mac the first launch is right-click → Open. The npm `@tauri-apps/{api,plugin-opener,plugin-dialog}` are pinned to the Rust crates' minor versions (the CLI refuses to build on a mismatch).

---

## D-025 — Review: edits reach the minutes, re-runs keep edits, clean-up actions
_2026-10-24_

**Decision** (P3.3, user-approved plan):
- **Edits flow back into the lists.** Each summary section carries `edited`. Saving Key points / Decisions / Action items / Open questions rebuilds the list behind it from the bullets (`modules/meetings/review.rs`): Key points = "- **Topic**" + nested points; Action items = "Owner — task — due" (missing parts → "Unassigned" / "Not set"); Decisions and Open questions keep `decided_by`/`condition` and `who`/`by` when the text still matches (case-insensitive). Anything off-format is kept as text, never dropped. The minutes therefore always match the Summary tab (closes D-024's known limit). Chosen over a structured row editor: smaller, stays plain text; revisit if bullets prove fiddly.
- **Re-run keeps edits by default.** With edited sections the UI asks Keep my edits / Discard edits; kept sections (and their lists) replace the fresh ones. The automatic summary after a re-transcription keeps edits too. `edited_at` now means "some section is edited".
- **Transcript corrections** update the segment in place (FTS follows via the update trigger). `SummaryDoc.generated_at` + segment `updated_at` → `transcript_changed` on the Summary tab. Re-transcribing still replaces the whole transcript (corrections included).
- **Notes** (`meeting.notes_md`, autosaved) are appended to the meeting text under "[Notes from the meeting owner]"; the prompt treats them as reliable data (still data, not instructions).
- **Rename** re-renders the email subject. **Delete meeting** moves `recordings/<id>/` to the Trash (or an audio file only if it lives under `recordings/`), then deletes the row (transcript, summary and export rows cascade); exported DOCX/PDF files stay in `exports/`. Blocked while recording/transcribing/summarising. **Delete export** moves its files to the Trash.
- Summaries edited before P3.3 (only `edited_at`, no section flags) can't be kept section by section: Re-run asks before replacing them.
- No migration (columns existed; flags live in the summary JSON).

---

## D-024 — Minutes export: structured summary, PDF only on request, converter chain
_2026-10-24_

**Decision** (user-approved D2 for P3.2; "PDF is an option, never forced"):
- **One summary call, richer JSON**: attendees (organisation, job title, attendance), discussion per topic, decisions with `decided_by` / `condition`, open issues with `who` / `by`, `next_focus`. The email's Key points / Decisions / Action items / Open questions sections are rendered from these lists (Key points now grouped by topic). Older summaries still load (decisions stored as plain strings are read as `{text}`). Live: 42 s for the test meeting (was 24 s).
- **Model** `kernel/export/model.rs` maps meeting + summary onto the template fields (docs/document-export.md §2.2): local-time date line "Friday, 24/10/2026 — 10:00–11:30" (chrono `Local`), reference `BB-DDMMYYYY-NN` where NN is the meeting's position among meetings held the same local day (no stored counter), attendees grouped by organisation. Numbering prefixes ("3.", "QĐ-", "CV-") live in the template; data gives `n` and `nn`.
- **Exports are explicit**: nothing is exported automatically. `export_minutes_docx` writes `exports/<yyyy-mm-dd> <title>.docx` (never overwrites: " 2", " 3"…) and a `document_export` row; `export_minutes_pdf` converts an export's DOCX **as it is on disk**, so edits made in Word are kept. The Minutes tab offers Word or Word + PDF (Word by default).
- **PDF** `kernel/export/pdf.rs`: Microsoft Word → LibreOffice → Pages, first found and working; 60 s timeout, process killed on drop; paths passed to AppleScript as `argv`. Word and Pages work inside their own sandbox container (no file-access prompt); Word is quit afterwards only if the export launched it. Measured: Word 10.5 s including launch. One conversion at a time (a process-wide lock); the Word script addresses its document by a unique file name, never "the active document". Stored export paths are only opened/converted when they resolve inside the workspace's `exports/` folder.
- **Template lookup**: the workspace's `brand/templates/meeting-minutes/template.docx`, else the bundled starter pack with the same slug (until tenant setup copies packs into workspaces).
- `NSAppleEventsUsageDescription` added to Info.plist and `com.apple.security.automation.apple-events` to the entitlements (needed for hardened-runtime builds, P4.1).

**Known limits**: hand edits to Key points / Decisions / Action items / Open questions on the Summary tab change the email only — the minutes use the structured lists (to be solved with review/editing, P3.3). LibreOffice and Pages paths are untested on this Mac (not installed). The first PDF may show macOS prompts: control Microsoft Word (Automation) and possibly "access data from other apps" (writing into Word's container).

---

## D-023 — Minutes DOCX: template engine, VIFC placeholder template, unknown fields stay as hints
_2026-10-24_

**Decision** (user-approved D1–D4 for P3.2):
- **Engine** `kernel/export/docx.rs`: a minimal DOM over quick-xml that keeps every tag's raw source, so everything the engine doesn't fill is written back byte for byte. Template language in plain Word text: `{{field}}` (newlines → line breaks) and `{{#list}} … {{/list}}` blocks — a paragraph repeats when both markers are in it, a row when they're in different cells, a range of paragraphs/rows when they're in different elements; tables never repeat as a unit; empty/missing lists remove the block; blocks nest. Placeholders split across runs are merged first (the run where the placeholder starts keeps its formatting). Document, headers and footers are filled; other zip entries are raw-copied; output written to a temp file then renamed.
- **The VIFC template is a hand-filled form** (bracket hints, worked examples, two "delete before issue" instruction boxes, no placeholders). `scripts/vifc_template_placeholders.py` produced `brands/vifc/templates/meeting-minutes/template.docx` from the original (kept as `original.docx`): only text is edited, run by run, so each run keeps its own formatting (bold labels, blue bullets/numbers); guidance lines, examples and instruction boxes are removed; one example row/paragraph is kept as the pattern to clone. Attendees are grouped by organisation; actions are not grouped by session (the summary doesn't group them).
- **Fields the AI can't know stay as the template's `[ ]` hints** (venue, chair, minute-taker, classification, relationship register, circulation table, signature names) for the user to complete in Word before issue.
- **Starter pack in the repo**: the original + placeholder template and the two VIFC logo SVGs are in `brands/vifc/` (company files in the user's private repo, user-approved).
- Verified by converting a filled sample with Microsoft Word (AppleScript, inside Word's sandbox container — 7.9 s, no prompts) and rendering the PDF pages: layout, header/logo, bilingual slogan, footer and page numbers identical to the template; agenda/bullets/open issues keep their mixed formatting; grouped attendees stay one table.
- Robustness (architect review): runs are merged before blocks are found; marker stripping fails rather than loops; empty blocks leave valid cells/tables; BOM stripped; XML-forbidden characters dropped. Item keys are looked up before outer keys, so templates avoid reusing outer names inside blocks.

---

## D-022 — Accent-insensitive Vietnamese search (FTS rebuild, user-approved)
_2026-10-24_

**Decision** (the user explicitly approved the DROP statements): migration `workspace/0002_fts_remove_diacritics.sql` rebuilds `transcript_fts` and `summary_fts` and their sync triggers:
- `tokenize = 'unicode61 remove_diacritics 2'` — the default (`1`) folds only single-diacritic letters; Vietnamese ế, ộ, ữ carry two. Now "tieng" finds "Tiếng".
- a hidden `folded` column with đ→d / Đ→D (đ is a separate Unicode letter, not d + a mark, so no tokenizer option folds it). MATCH searches every column; snippets use the original column (transcript `text`, summary `email_md`) so they show the real spelling — a word matched only through `folded` is found but not highlighted.
- Derived data only: both tables are refilled from `transcript_segment` / `summary` in the same migration; each FTS row takes its source rowid at rebuild time and `fts_rowid` is reset to match; later rows come from the recreated triggers (D-012 rules unchanged).

**Not covered**: meeting titles still use `LIKE` (accents and đ must be typed).

---

## D-021 — Summary: thinking off by default, prompt-shaped JSON, estimated tokens, auto after transcript
_2026-10-24_

**Decision** (user-approved; overrides "thinking ON" in ai-engine.md / the playbook for summaries):
- **Thinking OFF by default.** Measured on the M1 Pro with `qwen3.5:9b`: a toy 665-token transcript took **213 s** with thinking (4,240 reasoning tokens at 20 tok/s) vs **15 s** without, with the same accuracy. A per-run **"deep" option** (Re-run with thinking) stays for hard meetings. Real test: a 36-line partner meeting → complete email in **24 s**.
- **Gateway** `kernel/ai/ollama.rs`: `/api/chat`, streaming, `num_ctx` 32,768 always set, `keep_alive` 5 min, temperature 0.3; `TextSession` holds the AI queue for a whole job and unloads Whisper first; transcription asks Ollama to unload Qwen first (`keep_alive: 0`) — never two models resident. One `ai_log` row per call with real prompt/output token counts. `ollama_unavailable` / `ollama_model_missing` errors for a friendly UI state.
- **Token counting**: Ollama 0.40.9 has no `/api/tokenize` (404). Estimate `chars / 3` (measured 4.0 chars/token Vietnamese, 5.2 English) against an input budget of `num_ctx − answer reserve − 2,000`; if Ollama reports `prompt_eval_count ≥ num_ctx − 16` the gateway errors `context_overflow` and the summary is redone in parts. Map (chunks at half budget) → recursive reduce (groups that fit).
- **Structured output is unavailable**: `format` (schema or `"json"`) returns **501** for qwen3.5 on Ollama's MLX runner. The JSON shape is in the system prompt; the parser takes the outermost `{…}` (tolerates fences/prose) and does one repair round. The gateway keeps the `format` option for models that support it.
- **Prompt hygiene**: facts only; meeting text inside `<transcript>`/`<notes>`, declared data, instructions inside ignored; written directly in the meeting language (no translate pass); "Not discussed" for empty sections. Default structure (until meeting types): Overview · Key points · Decisions · Action items · Next steps · Open questions (Vietnamese titles for `vi`). Decisions and action items come back as structured lists and are rendered as "**Owner** — item — *due*".
- **Email** is a rendering of the summary (no AI call): `[Tenant] Title — d Mon yyyy`, greeting/sign-off per language, overview paragraph, sections in order, empty ones skipped. `pulldown-cmark` HTML with raw HTML escaped; clipboard gets HTML + Markdown text; `mailto:` percent-encoded.
- **Flow**: summary starts automatically after a successful transcript; `transcribed → summarising → summarised` (failure restores the previous status; launch recovery settles `summarising`). Upsert into `summary` (`ON CONFLICT DO UPDATE`, D-012); hand edits stamp `edited_at`, an AI run clears it. Notification "Summary ready".

---

## D-020 — Post-meeting flow: notification, FTS search, jump to the hit
_2026-10-24_

**Decision**:
- **Notification** (`tauri-plugin-notification`, Rust side only, no webview permission): "Transcript ready — <title>" or "Transcription failed — <title>" when a job ends; nothing on cancel. Clicking it brings New Dawn forward (opening the exact meeting would need notification actions — later).
- **Retry stays manual** ("Transcribe again"): transcription failures (missing model, unreadable file) don't fix themselves on a timer.
- **Search** (`search_meetings`, `?q=` on Meetings): title `LIKE` (escaped) + `transcript_fts` + `summary_fts`, title hits first, then FTS rank. User text is never FTS syntax: each word becomes a quoted phrase, the last one a prefix (`"acme" "cap"*`). Snippets mark matches with U+0002/U+0003 and are rendered as text with `<mark>`, never as HTML. Transcript hits carry the segment start; the result opens `/meetings/$id?t=<ms>`, which scrolls to the line and cues the player (no autoplay).
- **Accent-insensitive Vietnamese search** — resolved by D-022. (Original note: the FTS tables use the default `unicode61` tokenizer (`remove_diacritics 1`), which folds single-diacritic letters only, so "tieng" doesn't find "Tiếng" (searching with accents works). Fixing it means rebuilding both FTS tables with `remove_diacritics 2` in a new migration — a DROP/CREATE of derived index tables, which needs the user's explicit OK (CLAUDE.md).)
- The meeting page shows a **progress card** (stage, percent, Cancel, "you'll get a notification").

---

## D-019 — Recorder UI: toolbar pill, global start dialog, stop opens the meeting
_2026-10-24_

**Decision**:
- The toolbar's record pill is the one live control: **● Record** when idle; while recording a pulsing dot, elapsed (excluding pauses), two meters (you = mic, them = call, 60 dB scale from the 10 Hz `RecorderLevel`), pause/resume and ■ stop; "Saving…" while stopping.
- **Start dialog** (workspace, meeting type, title defaulting to "Meeting · <date, time>", language) opens from the pill, File → New Recording (⌘R) and the tray item (`MenuAction::ToggleRecording`: idle → open the dialog, recording → stop). Its open state lives in `AppLayout` (it belongs to the shell, not a page). Starting in another workspace switches to it.
- **Stop opens the meeting page**, where the queued transcription shows its progress.
- A **banner** above the page appears while recording when system audio is `silent` (with a button to Screen & System Audio Recording) or `unavailable` (mic only).
- **Settings → General → Permissions**: microphone state (Allow… / Open Settings) and a System audio row explaining it is checked while recording.
- Modal sheets share global classes (`.sheet`, `.sheetBody`, `.sheetHeading`, `.sheetActions`, `.formError`) in `components.css`.

---

## D-018 — Recorder: light callbacks, one mixer thread, crash-safe WAV
_2026-10-24_

**Decision**:
- **Callbacks do almost nothing** (rule 4): the cpal input callback and the Core Audio IO proc only mix to mono and `try_send` ~100 ms chunks into `std::sync::mpsc::sync_channel(50)` (bounded; a full channel counts a drop, never waits). Everything heavy runs on one **mixer thread**: rubato to 16 kHz, mic soft limiter (`tanh`), 80 Hz high-pass (RBJ biquad), `ebur128` short-term loudness gain towards −23 LUFS (adapts only while speech is present, +20 dB max), 600 ms window alignment, sum with proportional scaling above ±1, WAV write.
- **Mix directly at 16 kHz** (the playbook said 48 kHz internally): the WAV and Whisper both want 16 kHz mono, so one resample per source is enough.
- **Mic soft limiter**: Core Audio float input is not clipped — `say` through the MacBook speakers measured **4.7× full scale** at the mic. `tanh` keeps speech levels unchanged (< 3 % below 0.3) and bounds feedback, so the other side isn't scaled down by mic peaks. **Headphones are still recommended** (no echo cancellation in v1).
- **System audio**: `cidre` 0.29 global mono process tap in a private, auto-starting aggregate device (macOS 14.2+). No query API exists for the System Audio Recording permission; exact zeros for 3 s while the mic hears speech → `SystemAudio::Silent`. If the tap can't be created, recording continues mic-only (`Unavailable`).
- **Threads**: a capture thread owns the non-`Send` `cpal::Stream` and the tap and waits for Stop; dropping them closes the channels, the mixer drains and finalises the WAV.
- **Crash-safe WAV** (16 kHz mono 16-bit): the header sizes are rewritten and the file synced every 2 s, so a killed app leaves a playable file; at launch `recover_recordings` repairs the header from the file length and marks the meeting `recorded` (`failed` if unrecoverable). App exit stops the recorder before the databases close.
- **Session**: Idle → Recording ⇄ Paused (writing stops, meters stay live, elapsed excludes pauses) → Stopping → Idle. Start creates the meeting as `recording`; Stop marks it `recorded` and queues transcription. Events `RecorderStatusChanged`, `RecorderLevel` (10 Hz).
- **Permissions**: microphone via `AVCaptureDevice` authorisation status/request (cidre; the async variant isn't `Send`, so the completion handler feeds a channel on a blocking thread). System Settings links: `Privacy_Microphone`, `Privacy_ScreenCapture` (Screen & System Audio Recording).
- **Dev builds**: macOS attributes the permission prompts to the launching app (VS Code / Terminal) until the app is signed (P4.1).

**Measured**: live 3 s capture on the M1 Pro — tap at 48 kHz mono, both sides captured, 0 dropped chunks, WAV decodes to the exact sample count.

---

## D-017 — Core ML encoder: measured, not adopted (Metal only)
_2026-10-24_

**Measured** on the M1 Pro 16 GB, Whisper large-v3, 10.1 min of synthetic English speech (`say`, 54 VAD segments), `cargo test bench_transcribe` (dependencies optimised, D-015):

| Encoder | Transcribe | Load + 2 s warm-up |
|---|---|---|
| Metal (default build) | 64.5 · 63.5 · 63.2 s — ~63.7 s, 9.5× realtime | ~3 s |
| Core ML (`--features coreml`, `ggml-large-v3-encoder.mlmodelc`) | first run ever 146 s (macOS compiles for the Neural Engine); then 60.9 · 116.4 · 59.5 · 62.5 · 63.8 s — median ~62.5 s | ~6 s |

Whisper.cpp confirmed `Core ML model loaded` (no silent fallback). Same 54 segments either way.

**Decision**: stay Metal only. Core ML saves ~2 % at best, costs a 1.2 GB extra download per model, doubles load time, makes the first run 2× slower and showed an unexplained 2× outlier. The `coreml` cargo feature and the `bench_transcribe` test stay (off by default) so this can be re-measured on new hardware or with turbo.

**Also recorded**: whisper-rs 0.16 exposes whisper.cpp's built-in Silero VAD — used instead of `silero_rs`/ONNX Runtime (D-014). Extrapolated: a 30-minute meeting ≈ 3¼ min with large-v3 on Metal (turbo should be roughly twice as fast).

---

## D-016 — Import and meeting UI: URL-state sheet, progress in the query cache, asset-protocol audio
_2026-10-24_

**Decision**:
- **Import Audio** (File menu, ⌘⇧I, or the Meetings page button) opens a sheet at `/meetings?import=true` (URL state). The native file picker comes from `tauri-plugin-dialog` (`dialog:allow-open` only), called inside `src/kernel/bridge/meetings.ts`. The sheet picks workspace (default: current), meeting type (optional; the list is empty until tenant setup), title (default: file name) and language (en / vi / auto).
- **Live progress without a global store**: `useTranscriptionSync` (mounted once in the main window) writes `TranscriptionProgress` events into the TanStack Query cache (`setQueryData`); rows and the meeting page read it with a disabled query. `TranscriptionFinished` clears it, records a non-cancel error, and invalidates the meeting, list and transcript.
- **Meeting page** `/meetings/$meetingId?tab=transcript|summary|minutes`. The `<audio>` source is the copied file through the asset protocol (scope `$APPDATA/**`, already allowed by CSP `media-src`); clicking a timestamp seeks and plays; the row under the playhead is highlighted. Summary and Minutes render their structure as ghosts until P3.
- The import sheet is a native `<dialog>` (`showModal`: focus trap, inert background, Esc) using `.glass` — a modal sheet is added to the skill's glass list (drawer, popovers, quick capture, menus). Importing into another workspace switches to it first.
- **Asset protocol scope narrowed** to `$APPDATA/workspaces/*/recordings/**` (+ `$RESOURCE/**`): databases and models are not reachable from the webview. Brand logos will add `brand/logo/**` when P1.4 lands.
- **Webview CSP no longer allows Ollama** (user decision): `connect-src` is `'self' ipc: http://ipc.localhost` only. All AI goes through Rust (`kernel/ai`), so rule 2 is enforced by the webview too.
- **Whisper idle unload** is a registry setting (`whisper_idle_unload_secs`, 30–3600, default 120) applied live; Settings → Models offers 1 / 2 / 5 / 15 min.

---

## D-015 — Transcription engine: whisper.cpp VAD, one job at a time, raw abort callback
_2026-10-24_

**Decision**:
- **Pipeline** (`modules/meetings/transcribe.rs`): copy the file to `recordings/<meeting-id>/audio.<ext>` → decode with symphonia (sample rate from the decoded stream) → mono, peak-normalise to 0.95, rubato sinc to 16 kHz, clamp → whisper.cpp **built-in Silero VAD** (bundled model; timestamps are centiseconds) → Whisper per speech segment with timestamps on, offset by the segment start → language-safe cleanup → one transaction (delete + insert segments, status `transcribed`, `duration_ms`) and an atomic `transcript.json`. Status: `recorded | failed | transcribed → transcribing → transcribed`; on any failure the status is set once: a cancel restores the previous status, an error becomes `failed` unless an earlier transcript exists. Meetings left `transcribing` by a quit/crash are settled at launch (`recover_interrupted`). `transcript.json` is fsynced and written after the database save (non-fatal side file).
- **Gateway** (`kernel/ai/whisper.rs::transcribe`): features call this one function, which takes the AI-queue turn, runs the feature's `prepare` (audio → 16 kHz) and Whisper in `spawn_blocking`, and writes exactly one `ai_log` row (ok / error / cancelled). `WhisperEngine::transcribe` is private to `kernel/ai`.
- **Engine** (`kernel/ai/whisper.rs`): one model in memory, loaded on first use (Metal, flash attention), dropped after **120 s idle** (checked every 30 s; setting in stage C). One `WhisperState` per job. Beam 3, temperature 0.2, entropy 2.4, logprob −1.0, no-speech 0.55, `suppress_blank`/`suppress_nst`, `n_threads = min(cores, 8)`, explicit language (`en`/`vi`/`auto`), never translate. Model choice: the first downloaded in catalogue order (turbo, turbo-q5_0, large-v3).
- **Cleanup**: drop annotation-only (`[Music]`, `(applause)`), known hallucinations (exact: "thank you for watching", "you"…; fragments: "amara.org", "ghiền mì gõ"…) and collapse a chunk repeated ≥ 4 times by **character n-grams** — "từ từ" survives, digits/punctuation never collapse.
- **whisper-rs 0.16 bug**: `FullParams::set_abort_callback_safe` stores `Box<Box<dyn FnMut>>` but its trampoline casts the pointer to the concrete closure type, so whisper.cpp calls garbage and the encoder fails (`-6`). We use the raw `set_abort_callback` + `set_abort_callback_user_data` with a pointer to the job's `AtomicBool`. Remove when whisper-rs fixes it.
- **One AI job at a time**: `kernel/ai/queue.rs` (`AiQueue`, an async mutex held for the whole job); a cancel while queued returns before any work. Locks recover from poisoning instead of panicking.
- **Dev builds optimise dependencies** (`[profile.dev.package."*"] opt-level = 3`): whisper.cpp (CMake), symphonia and rubato are unusably slow unoptimised; our crate stays at debug for fast rebuilds.
- `meeting.audio_path` is stored absolute, like `workspace.folder_path`.

**Measured** (M1 Pro, test build): 11 s of `say` speech with a 3 s pause → 2 VAD segments, word-perfect English, correct timestamps, 5.3 s with large-v3 including model load.

---

## D-014 — Model downloads: the one network exception; whisper.cpp's built-in VAD
_2026-10-24_

**Decision**:
- **Network exception (user-approved amendment to CLAUDE.md rule 3)**: `src-tauri/src/kernel/ai/models.rs` is the only code allowed to reach a non-localhost host. It fetches model files from `huggingface.co` at **pinned commits** (`ggerganov/whisper.cpp@5359861c…`, `ggml-org/whisper-vad@9ffd54a1…`), only when the user clicks Download. Hugging Face answers `/resolve/` with a redirect to its CDN, so redirects are followed only to `huggingface.co` / `*.hf.co` hosts (anything else stops the download). It renames a file into place only after its exact size and **SHA-256** match the catalogue. `scripts/check-localhost-only.sh` allows `https://huggingface.co/` in that one file only. The webview CSP is unchanged (downloads run in Rust).
- **Downloader**: `.part` file with HTTP Range resume (206 append, 200 restart, 416 verify), connect/read timeouts and a 30 s stall timeout, cancel noticed within 1 s, an early end of stream keeps the part, progress every 250 ms with B/s, cancel keeps the partial file, a hash mismatch deletes it. It runs as a background task; the UI follows `ModelDownloadProgress` / `ModelDownloadFinished` events.
- **Catalogue (downloadable)**: large-v3-turbo (default, 1.62 GB), large-v3-turbo-q5_0 (0.57 GB), large-v3 (3.10 GB).
- **Silero VAD v5.1.2 is bundled, not downloaded** (user request): the 0.9 MB MIT-licensed file is committed at `src-tauri/resources/models/` (source commit, SHA-256 and licence in its README; a unit test checks the hash) and shipped via `bundle.resources` as `Contents/Resources/models/`. Silero v6.2.0 exists at the same commit but is not used until the bundled whisper.cpp is confirmed to load it.
- **VAD**: whisper-rs 0.16 exposes whisper.cpp's built-in Silero VAD (`WhisperVadContext`, `WhisperVadParams`), so `silero_rs` and ONNX Runtime are not needed. Mapping from ai-engine §3a: threshold 0.5 (whisper.cpp derives the 0.35 release threshold), min speech 250 ms, min silence 2000 ms (= redemption for file import), max segment 25 s (splits at silence > 98 ms instead of "quietest window ±3 s"), one symmetric pad instead of 300/400 ms.
- Byte counts cross the bridge as `u32` (max 4 GiB) so TypeScript gets plain numbers; widen if a model outgrows it.

**Rationale**: in-app download with verification is safer and simpler than asking the user to fetch 1.6 GB files by hand; pinning + hashing means nothing unexpected can arrive.

---

## D-013 — Menus, tray and Settings: events, hidden main window, UI-owned shortcuts
_2026-10-24_

**Decision**:
- **Typed events** (tauri-specta) connect Rust and the UI: `WorkspacesChanged` (every window refreshes its queries), `MenuAction` (switch workspace, toggle sidebar), `OpenSettingsTab`. Menu items the UI doesn't own are handled in Rust (`kernel/menu.rs::on_event`).
- **Settings is a separate WebviewWindow** ("settings", 720×520, same native glass and dark scrim), served by the same bundle at `/settings?tab=`. Two webviews have two query caches, which is why `WorkspacesChanged` exists.
- **Current workspace** lives in `registry.db` `setting.current_workspace`; the hub falls back to the first active workspace if it is unset, archived or deleted.
- **⌘1…⌘9 are handled in the UI**, not as menu accelerators: the menu would otherwise be rebuilt on every workspace change.
- **The main window hides instead of closing** (the dock icon and "Open New Dawn" in the tray show it again; Quit exits and runs the hub's checkpoint-and-close). This keeps the tray item meaningful now and the recorder alive later.
- **`tauri-plugin-window-state` 2.x saves POSITION | SIZE only.** `VISIBLE` is excluded on purpose: restoring it would relaunch with a hidden main window.
- Menu items for unbuilt features (New Recording, Import Audio, Tenant Setup) are **disabled, not hidden**.
- The tray icon is a generated monochrome template PNG (`src-tauri/icons/tray.png`), a placeholder until the real icon in P4.1.

**Consequence**: later prompts enable those menu items by changing the `enabled` flag in `menu.rs` and adding a case in `on_event` (or a `MenuAction` variant for UI-owned actions).

---

## D-012 — Data kernel: error model, pool opening, FTS linking, generated bindings
_2026-10-24_

**Decision**:
- **Errors**: kernel code returns `anyhow::Result` and raises `AppError { code, message }` (`invalid_input`, `not_found`) where the UI needs a code. `From<anyhow::Error>` recovers it at the command boundary; anything else is logged and becomes `internal`.
- **Opening databases**: `hub::open_pool` is the only function that opens a SQLite file (WAL, `foreign_keys`, `busy_timeout` 5 s, migrate). Registry and workspace pools both go through it. `WorkspaceHub` owns the `Registry`; commands talk only to the hub.
- **`setting` tables** use `key TEXT PRIMARY KEY`, a named exception to the UUID-id rule. Nothing references a setting row.
- **FTS5**: the FTS tables hold their own copy of the text. Each source row stores its `fts_rowid`, set by the insert trigger, and sync triggers look up by it. TEXT-PK tables' implicit rowids can change on `VACUUM`, so external-content FTS is unsafe here. **Rules for later code**: never write `fts_rowid` from app code, and never `INSERT OR REPLACE` / `REPLACE` into `summary` or `transcript_segment`, because that skips the delete trigger and leaves stale search rows. Use `INSERT … ON CONFLICT … DO UPDATE` for summaries, and delete + insert for re-transcription.
- **Timestamps**: written only by SQLite `strftime('%Y-%m-%dT%H:%M:%fZ','now')`, so there is one format and `chrono` isn't needed yet.
- **Bindings**: `src/bindings.ts` is generated by tauri-specta on every debug launch and by `cargo test export_bindings`, and it is committed. The UI calls `commands.*` only through `src/kernel/bridge/` (`unwrap()` → `BridgeError`). This supersedes D-010's hand-typed bridge.

**Rationale**: One error type at the boundary without an enum per module. One function to audit for pool settings. Search stays correct after VACUUM and re-runs.

**Consequence**: the tauri-specta, specta and specta-typescript versions are pinned with `=` and must be upgraded together. The migration-safety agent rated 0001 (registry + workspace) SAFE after the `fts_rowid` change.

---

## D-011 — Neutral dark scrim behind the shell in dark mode
_2026-10-24_

**Decision**: In dark mode a neutral `--shell-scrim` (black at 40 %) is painted behind the sidebar and toolbar. It's transparent in light mode. Brand-coloured text uses the derived `--brand-text` (`--brand-400` in dark), and text on brand fills uses `--on-brand`.

**Rationale**: Over a bright wallpaper, the Regular Liquid Glass stays light in dark mode, so light text on it lost contrast. A CSS scrim follows appearance changes instantly. Tinting the native glass would mean clearing and reapplying it from Rust on every theme change. The scrim is neutral, not brand, so the "never tint glass with the brand" rule holds.

**Consequence**: Tune contrast with the one `--shell-scrim` value. Switch to a native `tint_color` only if the scrim looks wrong next to native apps.

---

## D-010 — Shell glass behind the whole window; code-based routes; interim hand-typed bridge
_2026-10-24_

**Decision**: `kernel/glass.rs` applies one native glass layer behind the whole window (`apply_liquid_glass`, Regular, radius 12 = `--radius-window`, untinted; on failure `apply_vibrancy` Sidebar). There's no OS-version check: the crate returns `UnsupportedPlatformVersion` below macOS 26 and we fall back. The result is recorded once and read by `glass_status()`. Routes are defined in code in `src/router.tsx` (no file-based router plugin); toolbar titles come from route `staticData`. Until tauri-specta arrives (P1.2), `src/kernel/bridge/*.ts` wrappers are hand-typed, and `AppError` is a minimal `{ code, message }` struct.

**Rationale**: The DOM decides what reads as glass (transparent sidebar/toolbar) versus solid (`.content-root`), so one native layer is enough and there are no per-region native views to keep in sync. Code-based routes avoid a Vite plugin and generated route files for a handful of pages.

**Consequence**: P1.2 replaces the bridge wrappers with generated `bindings.ts` and extends `AppError`. If a region ever needs a different native material, add a second effect view then.

---

## D-009 — Brand import: craft the template elsewhere, drop it in, extract, review, reuse
_2026-10-23_

**Decision**: A tenant's brand and minutes template are imported from files the owner designs elsewhere (a `.docx` template, optionally a brand guideline PDF, logo and font files). Deterministic parsing of the DOCX (theme fonts/colours, styles, header image and text, headings and placeholders) plus Qwen3.5-9B reading the guideline PDF produce a proposal the owner reviews before it is saved as `tokens.json` + template `manifest.json`.

**Rationale**: The design work already happens in Word/brand tools; the app should read it, not reinvent it. Review-before-save keeps the AI honest.

---

## D-008 — Meeting type is the unit of configuration
_2026-10-23_

**Decision**: Each tenant defines meeting types (Partner call, Internal sync, …). A meeting type owns its minutes `.docx` template, its **summary structure** (ordered sections with instructions, pre-filled from the template's headings) and its email format. The summary prompt is generated from the structure; the email body is rendered from the summary (no second AI call); the minutes document is the template filled with the same sections.

**Rationale**: One source of truth per meeting kind; the document the team receives and the summary they read in email never drift apart.

---

## D-007 — v1 scope: tenants + the meeting machine; AI kept simple; no live transcription
_2026-10-23_

**Decision**: v1 = tenant setup (brand, meeting types, templates) + record (mic + system audio, no bot) → batch transcript after Stop → executive summary as email body → branded minutes DOCX/PDF. One Whisper model and one Ollama model (`qwen3.5:9b`, thinking on), run one job at a time; no tiers, no scheduler, no live transcription. Tasks/cockpit, notes, knowledge modules, research, speaker labels and bundled Ollama are deferred.

**Amends D-004**: Be Vietnam Pro remains the only UI typeface; a tenant's `document_fonts` (heading/body) apply only to exported documents.

**Rationale**: It is the loop that pays back first (every meeting produces an email and a document), and removing live transcription removes the only reason two models would ever run at once on 16 GB.

---

## D-006 — Document export: fill the workspace's designed .docx template, convert to PDF with whatever Office app is installed
_2026-10-23_

**Decision**: Meeting minutes (and later dossiers and briefs) are produced by filling the tenant's own designed `.docx` template in Rust (zip + XML, run-merging, repeating rows, Markdown → template styles), then converted to PDF through a runtime chain — Microsoft Word via AppleScript, else LibreOffice headless, else Pages; a Typst-rendered fallback comes later. House style: English first, Vietnamese second, one line each in headers and footers.

**Rationale**: The company's template is the design; recreating it in HTML would drift from the Word file people also receive. Converter detection keeps the app local and dependency-free when an Office app is present. See `docs/document-export.md`.

**Consequence**: `brand/templates/` is part of every tenant; tables `document_template` and `document_export`.

---

## D-005 — Meetings recorded in-app, no bot
_2026-10-23_

**Decision**: New Dawn records meetings itself — microphone via `cpal` plus the call's system audio via a Core Audio process tap (`cidre`, macOS 14.2+) — with live Whisper transcription. Nothing joins the call. File import is the secondary path.

**Rationale**: Meetily ships exactly this stack and proves it works on macOS without a virtual audio device. See `docs/meetily-teardown.md` §2.7 and §6 for what to copy and what to fix (bounded channels, one reused Whisper state, no English-translate default, language-safe cleanup).

**Consequence**: Phase 3 includes the recorder (P3c-i/ii/iii). Recordings are 16 kHz mono WAV in v1; M4A via an `ffmpeg` sidecar later.

---

## D-004 — Apple owns the shell, the brand owns the content; one typeface
_2026-10-23_

**Decision**: The window chrome (background, sidebar, toolbar) is native Liquid Glass and follows macOS 27 Golden Gate conventions. A workspace's brand is applied only to the content area and only through four knobs: logo, colours, corner-radius style, button style. **Be Vietnam Pro is the single typeface for every tenant** and is not brandable.

**Rationale**: Two things want to own the look — Apple's glass and the company's brand. Keeping the shell native makes the app feel first-party and honours the user's glass-intensity and accessibility settings; keeping the brand in content tokens makes switching workspaces a data change, not a code change. One typeface keeps every workspace reading as the same app and is Vietnamese-ready.

**Consequence**: No per-workspace fonts. `brand/tokens.json` schema `newdawn-brand-tokens/v1` has no type field. See `docs/plan.md` §5.

---

## D-003 — 100 % local AI; no cloud code path
_2026-10-23_

**Decision**: Whisper (`whisper-rs`, Metal) compiled into the app; Qwen through Ollama (installed, v1) → bundled Ollama sidecar (v1.5) → own `llama-cpp-2` helper (v2, only if needed). There is no cloud provider, no API key, no toggle. `scripts/check-localhost-only.sh` fails the build on any non-localhost URL.

**Rationale**: Client and partner content never leaves the Mac; nothing to configure or leak. The runner options are interchangeable behind `kernel/ai/`, so starting with installed Ollama costs nothing later. See `docs/ai-engine.md`.

**Consequence**: On a 16 GB M1 Pro the quality tier is Qwen3-8B Q4 and the fast tier Qwen3-4B; one heavy model at a time, scheduled by the gateway.

---

## D-002 — One SQLite file per workspace, plus registry.db
_2026-10-23_

**Decision**: Each workspace lives in `~/Library/Application Support/co.majorisdigital.newdawn/workspaces/<slug>/workspace.db` with its attachments and brand pack beside it. A small `registry.db` lists workspaces. Every workspace DB has the identical schema (`migrations/workspace/`); the registry has its own (`migrations/registry/`). Cross-workspace views fan out across open pools in `kernel/hub.rs`.

**Rationale**: A company's data is one folder — back up, hand over, lock or delete as a unit; no accidental cross-tenant queries; matches the old one-project-per-company model without clone-and-diverge.

**Consequence**: Two migration sets; all data access goes through the WorkspaceHub; cross-workspace links are `(workspace_slug, id)` pairs and are rare by design. See `docs/plan.md` §4.

---

## D-001 — Tauri 2 with a Rust engine and a React cockpit
_2026-10-23_

**Decision**: Tauri 2. Rust owns the engine room (database, audio, AI, files, native integration); React + TypeScript + CSS modules own the screens. All-Rust UI frameworks (Dioxus, Iced, Slint, GPUI) rejected.

**Rationale**: Rust where it earns its keep; the UI skills and patterns from the three reference projects (CSS modules, ghost pattern, section rhythm, URL-state filters) carry over; Claude Code is most reliable on this split. See `docs/plan.md` §6 and `docs/implementation-plan-v0` §2 for the full argument.

**Consequence**: `whisper-rs` builds whisper.cpp with CMake at `cargo build` time; expect a slow first build.
