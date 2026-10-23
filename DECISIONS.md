# Architectural Decisions

Newest first. Every entry: what was decided, why, and what it means for future work. Keep entries short; link to `docs/` for the long version.

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
