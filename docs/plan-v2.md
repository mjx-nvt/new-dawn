# New Dawn — Plan v2 (v1 build scope: tenants + the meeting machine) — superseded by `plan.md` v3 on 2026-10-30, kept as history

_2026-10-23. Supersedes `plan v1` (kept in the Project as history). Decisions carried over: Tauri 2 + Rust engine + React cockpit; one SQLite file per tenant; Liquid Glass shell on macOS Golden Gate; DM Sans for the app UI (D-033; was Be Vietnam Pro); 100 % local AI; bundle id `co.majorisdigital.newdawn`; MacBook Pro M1 Pro 16 GB. Reference docs: `ai-engine.md`, `meetily-teardown.md`, `document-export.md`, `model-family-comparison.md`, `reference-audit.md`. Build prompts: `playbook.md`. Plain language._

## 1. What v1 is

**One sentence:** a native Mac app where each company you work for is a *tenant* with its own database and brand, and where you can press Record on any call, and after Stop get a full transcript, a detailed executive summary you can paste into an email, and a designed meeting-minutes Word document (and PDF) in that company's brand — all on your Mac, no bot, no cloud.

**Five things it does, in order of use**
1. **Tenant setup** — create a workspace (name, slug), import its brand (logo, slogan, key colours, document fonts) and its meeting types, each with a brand-aligned minutes `.docx` template. Done once per company.
2. **Record** — mic + the call's system audio, no bot in the call. A level meter, elapsed time, pause/stop. No live transcript.
3. **Transcript** — after Stop, Whisper transcribes the whole recording (English default, Vietnamese selectable) with timestamps.
4. **Executive summary** — Qwen writes a detailed summary that follows the meeting type's template structure; rendered as an **email body** (subject + text) with a one-click copy.
5. **Minutes document** — the same content poured into the tenant's designed `.docx` template, then a PDF via Word/LibreOffice/Pages; both saved under the tenant's exports and revealed in Finder.

**Everything else waits** (tasks/cockpit, notes, knowledge dossiers, research, live transcription, speaker labels, bundled Ollama). The architecture leaves room for them; v1 does not build them.

## 2. Architecture (unchanged shape, smaller surface)

```
┌──────────────────────── New Dawn.app ─────────────────────────────┐
│ Cockpit (React over native glass): Sidebar · Toolbar (● Record)    │
│   Meetings list · Meeting page (Transcript | Summary | Minutes)    │
│   Tenant setup wizard · Settings (Models, Converters, Templates)   │
│                     ▲ typed bridge / events ▼                      │
│ Engine room (Rust)                                                 │
│   registry.db · WorkspaceHub (one pool per tenant)                 │
│   recorder/  mic + Core Audio tap → mixer → incremental WAV        │
│   ai/        whisper.rs (whisper-rs) · ollama.rs (qwen3.5:9b)      │
│   brand/     tokens, import (docx parser + Qwen reading a PDF)     │
│   meetings/  transcribe job · summarise job · email body           │
│   export/    docx template fill · pdf converter chain              │
└────────────────────────────────────────────────────────────────────┘
```

**Keep the AI simple (like Meetily).** Two functions: `transcribe(wav, language) → segments` and `complete(prompt, think) → text`. One Whisper model (`large-v3-turbo`), one text model (`qwen3.5:9b`, thinking on). They never run at the same time — except a Live session (0.7.2, D-041), which holds the AI queue and keeps Whisper and the fast tier (`qwen3.5:4b`, thinking off) loaded together until Stop — so no scheduler, no tiers beyond that, no fallbacks. Every call is still logged (`ai_log`) and nothing but `localhost` is ever contacted.

## 3. Tenants

One folder and one database per tenant, as before:
```
~/Library/Application Support/co.majorisdigital.newdawn/
  registry.db
  models/                           ← Whisper models (Ollama keeps its own)
  workspaces/<slug>/
    workspace.db
    brand/  tokens.json  logo/  fonts/  templates/<meeting-type>/template.docx + manifest.json  guideline/<original uploads>
    recordings/<meeting-id>/audio.wav  transcript.json
    exports/<date> <title>.docx|.pdf
```

**Brand pack v2 — `brand/tokens.json`** (`newdawn-brand-tokens/v2`)
```json
{ "name": "VIFC Fintech Hub", "slug": "vifc",
  "logo": { "primary": "logo/….svg", "on_dark": "logo/….svg" },
  "slogan": { "en": "The convergence of global capital and Vietnamese fintech", "vi": "Nơi hội tụ dòng vốn toàn cầu và fintech Việt Nam" },
  "colors": { "primary": "#0132AD", "secondary": null, "accent": null, "text": "#0B1220", "canvas": "#FFFFFF",
              "status": { "positive": "#1F7A4D", "warning": "#9A6A00", "negative": "#B3261E" } },
  "document_fonts": { "heading": "Playfair Display", "body": "Be Vietnam Pro", "files": ["fonts/…"] },
  "radius": { "preset": "soft" }, "button": { "shape": "rounded", "primary": "filled" } }
```
- ~~`colors`, `logo`, `radius`, `button` skin the **app's content area**~~ — superseded by D-033 (25 Oct 2026): the pack is for exported documents only; the app's palette is Majoris orange on Slate and the UI typeface is DM Sans.
- `document_fonts` are used **only in exported documents** — the minutes `.docx` keeps the template's own fonts; the fallback PDF renderer (later) uses these.
- `slogan` appears in document headers (EN line, then VI line, one line each — house style).

**Meeting types — `meeting_type` table**, set up in the tenant wizard: name (Partner call, Internal sync, Delegation visit, TMO briefing…), default language, minutes template, **summary structure** (ordered sections with a one-line instruction each — derived from the template's headings and editable), email format (subject pattern, greeting, which sections to include, sign-off), include-transcript default.

**Brand import (the "craft it elsewhere, drop it in" flow)**
1. You drop: the minutes `.docx` template (required), optionally the brand guideline PDF, logo SVG/PNG files, font files.
2. Deterministic extraction from the `.docx`: theme fonts (heading/body) and theme colours from `word/theme/theme1.xml`; Heading/Normal styles from `styles.xml`; header image → logo candidate; header text → slogan candidate; body headings and `{{placeholders}}` → the template's **section structure**.
3. Qwen3.5-9B (with vision) reads the guideline PDF pages and returns named colours, font names, logo rules, tone notes.
4. Both proposals are merged into a review form; you confirm or edit; the app writes `tokens.json`, `manifest.json` and copies the files. Reusable for the next tenant.

## 4. The meeting flow

| Step | What happens | Where |
|---|---|---|
| Start | Toolbar ● or ⌘R or menu-bar item → pick tenant + meeting type + title | `recorder/session.rs` |
| During | mic (cpal) + system audio (Core Audio tap, macOS 14.2+) → mixer → 16 kHz mono WAV written every 2 s; level meter, pause/resume | `recorder/` |
| Stop | WAV finalised; meeting row created (status `recorded`); transcription job queued | `meetings/` |
| Transcript | Decode → VAD (2000 ms) → split > 25 s → Whisper (one state, timestamps on) → `transcript_segment` rows + `transcript.json`; progress bar; status `transcribed` | `ai/whisper.rs`, `meetings/transcribe.rs` |
| Summary | Prompt = meeting type's summary structure + house rules; transcript chunked only if it exceeds the context; Qwen3.5-9B with thinking on; output = one Markdown block per section → `summary` row; status `summarised` | `ai/ollama.rs`, `meetings/summarise.rs` |
| Email body | Rendered from the summary: subject (`[Tenant] Title — date`), greeting, 1-paragraph overview, key points, decisions, action items (owner · due), next steps, sign-off; **Copy as email** puts plain text + HTML on the clipboard; optional **Open in Mail** (`mailto:`) | `meetings/email.rs` |
| Minutes | Template fill (`document-export.md`) → `.docx`; PDF via Word → LibreOffice → Pages; both under `exports/`; Reveal in Finder | `export/` |
| Review | Meeting page tabs: Transcript (seekable player) · Summary (editable, Re-run) · Minutes (preview PDF, Export again) | UI |

Import of an existing audio file (m4a/mp3/wav) enters the same flow at "Transcript".

## 5. Design language (unchanged)
Apple owns the shell (real Liquid Glass via `window-vibrancy`, edge-to-edge sidebar, unified toolbar, one radius, dark mode, accent colour, reduced-transparency honoured). ~~The brand owns the content area through the tokens above.~~ D-033: the app's palette is Majoris orange on Slate and the brand pack is for documents only. DM Sans is the app typeface. Record control in the toolbar and the menu-bar item. Ghost pattern on every detail page.

## 6. Data model (v1)

**registry.db** — `workspace` (id, slug, name, folder_path, archived, last_opened_at), `setting`.

**workspace.db** — `meeting_type` (id, name, language, template_id, summary_structure_json, email_format_json, include_transcript, is_default) · `document_template` (id, kind, name, folder, manifest_json) · `meeting` (id, meeting_type_id, title, held_on, language, audio_path, duration_ms, status: recording|recorded|transcribing|transcribed|summarising|summarised|failed, notes_md) · `transcript_segment` (meeting_id, seq, start_ms, end_ms, text) · `summary` (meeting_id, sections_json, email_subject, email_md, model, created_at, edited_at) · `document_export` (meeting_id, template_id, docx_path, pdf_path, converter, created_at) · `ai_log` · `setting` · FTS5 over transcript text and summaries.

(Tasks, notes, entities, research tables arrive with their phases — same migration set, appended.)

## 7. Decisions added or amended

| # | Decision | Choice |
|---|---|---|
| D22 | **v1 scope** | Tenants + record → transcript → executive summary (email) → branded minutes DOCX/PDF. No live transcription, no tasks/cockpit, no knowledge modules in v1 |
| D23 | **AI kept simple** | One Whisper model + one Ollama model (`qwen3.5:9b`, thinking on); `transcribe()` and `complete()` only; sequential, never concurrent |
| D7 amended | **Fonts** | App UI: DM Sans, fixed (D-033; was Be Vietnam Pro). Tenant `document_fonts` (heading/body) apply only to exported documents |
| D24 | **Meeting type is the unit of configuration** | Template, summary structure and email format hang off the meeting type; tenants define types in setup |
| D25 | **Brand import** | DOCX/PDF/logo/fonts dropped in → deterministic DOCX parsing + Qwen reading the guideline → reviewed → saved as tokens + template manifest |
| D26 | **Email body** | A rendering of the summary, not a separate AI call; copy as text + HTML; `mailto:` optional |

## 8. Phases (prompts in `playbook.md`)

| Phase | Prompts | Outcome | Effort |
|---|---|---|---|
| 0 Setup | — (`GETTING_STARTED.md`) | Toolchain, Ollama + `qwen3.5:9b`, Meetily clone, scaffold runs | ≈ 1 week |
| 1 Shell + tenants | P1.1 glass window · P1.2 registry/hub/bridge · P1.3 sidebar/toolbar/menus/settings · P1.4 tenant wizard + brand import + meeting types | VIFC exists as its own DB with its brand and a "Partner call" type whose template is your official minutes DOCX | ≈ 3 weeks |
| 2 Record + transcript | P2.1 Whisper engine + models page + file import · P2.2 recorder (permissions, mic, system tap, mixer, WAV) · P2.3 post-meeting transcription job + meeting page | Press ● on a Zoom call, ■ at the end, read the full transcript minutes later | ≈ 3 weeks |
| 3 Summary + minutes | P3.1 Ollama + summary per meeting type + email body · P3.2 minutes DOCX fill + PDF + export UI · P3.3 review/edit/re-run polish | A pasteable executive summary and a branded minutes PDF per meeting | ≈ 2–3 weeks |
| 4 Ship | P4.1 icon, notifications, onboarding, backup, `New Dawn.app` | Daily use | ≈ 1 week |
| Later | tasks + cockpit · notes · knowledge dossiers (the old Ecomobi data) · research + embeddings · live transcription · speaker labels · bundled Ollama · Typst PDF fallback | | |

Roughly **ten weeks of evenings and weekends** to the full meeting machine.

## 9. Working rules
THINK → ALIGN → SEE → BUILD; one prompt per sitting; `cargo check` → `pnpm tauri dev` → `/build-check` → `LAST_BUILD.md` → commit. All data through the hub; all AI through `ai/`; no network beyond localhost; recorder never blocks the UI; migrations append-only.

## 10. Open items
1. Confirm the font reading in §3 (UI fixed; documents per tenant).
2. Copy the VIFC logo SVGs and your official minutes template into `brands/vifc/` (the wizard can also import them later).
3. Which converter you have for PDFs: `ls /Applications | grep -iE 'Word|LibreOffice|Pages'`.
