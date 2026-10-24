# New Dawn — Document export (designed DOCX → PDF)

_Written 2026-10-23. The feature: once a meeting has a transcript and a summary (and later, when a dossier or research brief exists), produce a **designed Word document from the workspace's own template**, then a **PDF**, ready to send. Entirely local. Plain language first, then the build prompt (P3e)._

## 1. What it does, in one sentence

"Export → Meeting minutes" takes the meeting's summary, decisions, action items, attendees and (optionally) the transcript, pours them into the workspace's designed `.docx` template (header, footer, logo, fonts, bilingual labels), saves the Word file, and converts it to PDF — the same way you would by hand, minus the hand.

## 2. Design

### 2.1 Content first, template second
Every export starts from a **document model** (JSON) that the app builds from the meeting:
```
{ title, workspace:{name, tagline:{en,vi}}, meeting:{date, time, location, attendees[], language},
  summary_md, decisions[], actions:[{text, owner, due, status}], open_questions[],
  people_orgs[], transcript?: [{time, speaker?, text}], generated_at, app_version }
```
The same model later serves dossiers (Phase 4) and research briefs (Phase 5), so the export module is written once.

### 2.2 Templates belong to the workspace
```
workspaces/<slug>/brand/templates/
  meeting-minutes/
    template.docx      ← designed in Word; placeholders inside
    manifest.json      ← name, which fields it uses, bilingual rules, page size
  partner-brief/ …
```
The VIFC starter pack ships with a port of your official **TEMPLATE_Bien_ban_cuoc_hop** (logo + tagline header, Lầu 30 address + page numbers footer, Be Vietnam Pro body). Starter templates live in the repo under `brands/<slug>/templates/` and are copied into a workspace on creation; a workspace can upload more in Settings → Workspaces → Templates.

**Placeholders** (the only thing the template designer needs to know — as built in P3.2, D-023):
- Field: `{{name}}` — text, kept in the formatting of the run where it starts; newlines become line breaks. Dotted paths work (`{{a.b}}`). Inside a block, the item's own keys are looked up first, then the outer data (so an item key must not reuse an outer name — attendees use `job_title`, not `title`).
- Block: `{{#list}} … {{/list}}`. Both markers in one paragraph → the paragraph repeats; in different cells of one row → the row repeats; in different paragraphs/rows → that range repeats. Tables never repeat as a unit. Blocks nest (attendee groups → members, discussion items → points).
- A block's value decides how often it renders: a list → once per item; `true`, an object, a number or a non-empty string → once; missing, `false`, `null`, `""` or `[]` → removed (a table left with no rows is removed too).
- Filled parts: `word/document.xml`, headers and footers. Everything the engine doesn't touch is written back byte for byte.
- The VIFC template fields: `title`, `datetime`, `meeting_type`, `reference`, `agenda[n, topic]`, `summary`, `next_focus`, `attendee_groups[organisation, members[name, job_title, attendance]]`, `discussion[number, topic, points[text]]`, `decisions[id, text, decided_by, condition]`, `actions[id, item, owner, due, status]`, `open_issues[n, issue, who, by]`. Fields the AI can't know stay as the template's `[ ]` hints.
- Not built (YAGNI until a template needs it): logo swap by alt text (the template carries its own logo), Markdown-rendered rich fields, header/footer cell widening.

**Bilingual rule (house style):** English first, Vietnamese second; in headers and footers each language sits on its own single line; the engine widens header/footer cells so neither line wraps. The manifest records `bilingual: { order: ["en","vi"], one_line_each: true }` and the template check warns if a header/footer cell is too narrow for the longest value.

### 2.3 DOCX generation (Rust, no external app)
A `.docx` is a zip of XML. The export engine (`kernel/export/docx.rs`):
1. Copies the template; opens `word/document.xml`, `word/header*.xml`, `word/footer*.xml`.
2. **Merges runs** inside each paragraph before substituting — Word often splits `{{title}}` into `{{`, `title`, `}}` across runs; this is the classic bug in every docx-template engine and must be handled first.
3. Substitutes simple fields; swaps the logo image part; clones the action rows; renders Markdown (`pulldown-cmark`) into paragraphs using the template's existing style ids; removes optional blocks that are off.
4. Writes the new `.docx` atomically (temp + rename) into `workspaces/<slug>/exports/<yyyy-mm-dd> <title>.docx`.
Crates: `zip`, `quick-xml`, `pulldown-cmark`. No `docx-rs` needed for template filling (it stays an option for building documents from scratch later).

### 2.4 PDF generation — a converter chain, detected at runtime
Converting a Word file to a *faithful* PDF needs a Word-compatible layout engine. The app tries, in order, whatever is installed:
1. **Microsoft Word** (most likely on your Mac): drive it with AppleScript via `osascript` — open the `.docx`, "save as PDF", close. Pixel-faithful to the template. ~3–5 s.
2. **LibreOffice** (free): `soffice --headless --convert-to pdf --outdir <exports> <file.docx>`. Very good fidelity; fonts must be installed (Be Vietnam Pro is bundled with the app and registered for the user on first run).
3. **Pages** (free on Macs): AppleScript export to PDF. Acceptable fidelity, weaker on complex tables.
4. **Built-in fallback (later, v1.5): Typst** — a pure-Rust typesetter embedded in the app; renders the same document model through a Typst twin of the template with the brand tokens. Not pixel-identical to Word, but always available and beautiful. Also the path to "PDF without Word" if New Dawn is ever handed to someone without Office.
The Settings → General page shows which converters were found; the export dialog shows which one will be used. No converter found → DOCX only, with a one-line explanation.

### 2.5 UI
> **As built in P3.2 (D-024):** the Minutes tab has Word / Word + PDF (Word by default; a PDF is never made unless asked), a file name and Export; an Exports list (Open, Open PDF or Make PDF from the saved DOCX, Reveal in Finder) and an in-app preview of the newest PDF. Settings → Converters lists Word / LibreOffice / Pages. Template picker, language mode, include transcript/attendees options, Copy path and Template check are not built yet. Word measured ~10 s including launch.

- Meeting page → toolbar **Export ▾** → "Meeting minutes (Word)", "Meeting minutes (PDF)", "Both". Dialog: template picker, language (EN / VI / bilingual), include transcript, include attendees, file name; **Preview** renders the PDF in-app (via the asset protocol) when a converter exists.
- Finder-style result row with "Reveal in Finder", "Open", "Copy path"; a `document_export` record per export (template, converter used, file paths, duration) so re-exports are one click.
- Settings → Workspaces → Templates: upload `.docx`, run **Template check** (lists placeholders found, missing/unknown fields, run-split placeholders fixed automatically, header/footer width warnings), set as default for a document kind.

### 2.6 Data
- `document_template` (workspace-scoped): id, kind (meeting-minutes | partner-brief | …), name, folder, manifest_json, is_default.
- `document_export`: id, source_kind, source_id, template_id, language_mode, include_transcript, docx_path, pdf_path, converter, duration_ms, created_at.

### 2.7 Why not generate the PDF directly from HTML?
It would be quick (print the web page), but it would ignore the Word template the company actually uses, and the result would not match the `.docx` you also send. Word-faithful PDF is the whole point; HTML-to-PDF stays a non-goal.

## 3. Where it sits in the plan

**P3e — Document export (≈ 1–2 weeks), right after P3d.** It completes the meeting loop: record → transcript → summary → minutes document → PDF. Phase 4 and 5 reuse it for dossier and research-brief exports (one new template each, no new engine).

Decision **D-006**: DOCX by template filling in Rust; PDF through a runtime converter chain (Word → LibreOffice → Pages), Typst as the built-in fallback later. Bilingual house style enforced by the engine.

## 4. Build prompt — P3e

```
Read CLAUDE.md, docs/document-export.md (all), docs/plan.md §9, and the newdawn-ui-patterns skill. Goal: "Export → Meeting minutes" produces a designed .docx from the workspace's template and a PDF via an installed converter.

Build:
1. `kernel/export/model.rs`: the document model (serde) and `from_meeting(meeting_id)` that assembles it from the hub (summary, decisions, actions, attendees, transcript segments).
2. `kernel/export/docx.rs`: template engine over `zip` + `quick-xml`: run-merging per paragraph; simple field substitution in document/header/footer parts; image swap by alt text `{{logo}}`; repeating table rows `{{#actions}}…{{/actions}}`; optional blocks `{{#transcript}}…{{/transcript}}`; Markdown → paragraphs using the template's style ids (`pulldown-cmark`); bilingual rule (EN then VI, one line each in header/footer; widen the cell or warn). Atomic write to `workspaces/<slug>/exports/`. Unit tests with a fixture template covering split placeholders and a 3-row action table.
3. `kernel/export/pdf.rs`: converter chain detected at startup — Microsoft Word (AppleScript via `osascript`), LibreOffice (`soffice --headless --convert-to pdf`), Pages (AppleScript) — each with a 60 s timeout and clear errors; `convert(docx_path) -> pdf_path`; report which converter was used.
4. Migration 0004: `document_template`, `document_export` per docs/document-export.md §2.6; starter template `brands/vifc/templates/meeting-minutes/` (port of TEMPLATE_Bien_ban_cuoc_hop: header logo + tagline EN/VI on two single lines, footer with the Lầu 30 address and page numbers, Be Vietnam Pro styles) and a neutral `brands/default/templates/meeting-minutes/`; copied into a workspace on creation.
5. Commands (thin): `export_meeting_docx`, `export_meeting_pdf`, `list_templates`, `check_template`, `set_default_template`, `list_converters`.
6. UI: Export ▾ menu on the meeting page with the dialog from §2.5 (template, language mode, include transcript/attendees, file name, preview when a converter exists); result row with Reveal in Finder / Open; Settings → Workspaces → Templates tab (upload, Template check, set default); Settings → General shows detected converters.
Constraints: no network; converters only via local processes; the engine never modifies the starter template in `brands/`. Done when: a recorded meeting exports to a .docx that opens in Word looking exactly like the VIFC template with the content filled in, and the PDF (via Word or LibreOffice) matches it; the bilingual header is two single lines; a template with a split `{{title}}` placeholder still fills correctly; `/build-check` passes.
```
