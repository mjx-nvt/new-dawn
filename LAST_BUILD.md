# Last Build — Storage, OTA updates, release 0.2.0
_2026-10-25_

## What changed
- **Attendees** (Summary tab → Attendees → Edit): the editor is now a sheet with a proper table (Name · Job title · Organisation · Attendance · Remove, Add attendee, Remember). Saving **fixes the corrected names everywhere** in the summary (all sections, owners, decisions, questions), hence the email and the minutes too, and updates the people memory; a **Re-run with corrected names** button follows for when the AI should rephrase. D-032.
- **Workspace badge**: logo only (name as tooltip; click opens the switcher).
- Review fixes on the rename: one pass, longest spelling first, correct names protected (so "Minh" → "Đỗ Minh" never becomes "Đỗ Đỗ Minh" on a second save; tested); accent-only and case-only corrections apply too; typed names are cleaned like the memory's; the sheet closes the native dialog (focus returns to Edit).
- **Storage** (Settings → Storage): once transcribed, a recording is compressed to AAC (about 20 MB an hour instead of 115) and the original removed — automatic from now on; **Compress now** does it for older ones. **Keep recordings for** 30 / 90 (default) / 180 / 365 days / forever after the summary; expired audio goes to the Trash, everything else stays (the Transcript tab says so). Sizes per workspace, the speech model and logs. `ai_log` pruned at 90 days. No schema change.
- **Updates** (Settings → About): **Check for updates**, release notes, **Install and restart**, daily check on launch (switchable; a banner in the main window when one is found). Signed with the new updater key; endpoint = the public releases repo.
- **Release 0.2.0**: `.github/workflows/release.yml` builds and publishes on a `v*` tag. Local `pnpm tauri build` produces the installable `.dmg` (see below).
- Crates: `tauri-plugin-updater`, `tauri-plugin-process` (+ their npm packages, pinned). D-031.
- Architect review — must-fixes applied: one compression at a time (the post-transcript one and "Compress now" could have written the same file); retention refuses to run when the recordings folder isn't reachable and only clears a meeting's audio pointer after the file was trashed or is confirmed gone. Should-fixes: the compressed file is synced to disk before the original goes; the update check times out after 15 s with a plain message; "Trash old recordings now" uses your retention setting (hidden when Forever); the date search no longer reads "Market" as March or "decision" as December; the release workflow runs on tags only and refuses a tag that doesn't match the app version; one bad file no longer stops Compress now; the launch check repeats daily while the app stays open in the tray.
- Meetings list filters/sort/date groups/date search (from the previous build) are part of this release.

## Releasing
- The source repo `mjx-nvt/new-dawn` is public (25 Oct); releases live there and the app reads `https://github.com/mjx-nvt/new-dawn/releases/latest/download/latest.json`.
- **Local** (what I do): `TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/newdawn.key)" pnpm tauri build`, then `RELEASE_NOTES="…" scripts/release-local.sh` → a GitHub release with the .dmg, the update bundle, its signature and latest.json.
- **CI** (optional): set the two secrets once — `gh secret set TAURI_SIGNING_PRIVATE_KEY --repo mjx-nvt/new-dawn < ~/.tauri/newdawn.key` and `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo mjx-nvt/new-dawn --body ""` — then pushing a tag `vX.Y.Z` that matches `tauri.conf.json` builds and publishes on GitHub's runner. **Back up `~/.tauri/newdawn.key`**: without it no future update can be signed.

## Manual steps (local)
1. Install `src-tauri/target/release/bundle/dmg/New Dawn_0.2.0_aarch64.dmg` (built here) → right-click → Open.
2. Settings → Storage: the two VIFC imports show "not yet compressed" → Compress now → sizes drop.
3. Settings → About → Check for updates: until the releases repo has a release it reports the check failed — expected.

---

# Brand packs live + Apple-style restructure
_2026-10-24_

## What changed
- **Meetings list: filters, sort, date groups, date search.** Grouped by Today · Yesterday · This week · Last week · Earlier this month · then month by month. Filter bar: Status (needs transcript / needs summary / summarised / failed), Type, When (7 days / 30 days / this year), Sort (newest / oldest / longest / title; non-date sorts switch grouping off). The search understands dates — `24 Oct`, `24/10/2026`, `Oct 2026`, `2026-10`, `yesterday`, `last week`, `this month` — alone or with words ("Oct 2026 Wired Score"). Filters live in the URL. All of it client-side over the list the page already has (`src/modules/meetings/listing.ts`, self-check `node --experimental-strip-types scripts/check-listing.ts`). Verified by screenshot.
- **Brand works and is editable.** The content area now follows the workspace's brand (VIFC = blue #0132AD, its logo in the sidebar). **Settings → Brand** (new pane, per workspace): live preview card; logo light / dark (SVG or PNG); primary, secondary, accent, canvas, text and status colours with the system colour picker; corners (sharp / soft / round), button shape and styles; slogan EN/VI; document fonts; **Reset to starter**. Saved to `<workspace>/brand/tokens.json` (the starter pack is copied in on first use).
- **App settings and workspace settings are separate.** The **Settings window** (⌘,) is app-only, System-Settings-style: App (General, Workspaces, Backup) · AI on this Mac (Models, PDF converters) · About, with grouped inset rows. The **Workspace page** in the main window (sidebar → Workspace, or Workspace → Workspace Settings… ⌥⌘,) holds the current company's own settings as tabs: Overview (name, folder, status) · Brand · Meeting types · People.
- **Type and spacing up**: body 14.5 px, section titles 17/600 sentence case, bigger gaps; uppercase only for the small brand eyebrow. Pages use the full panel width (the 880 px cap is gone).
- **Meeting page**: title (double-click to rename), facts row (date · length · language · status · type), **one primary button** for the current step (Transcribe → Write summary → Export minutes) and a **⋯ menu** (Rename, Transcribe again, Delete… in red). Tabs are a segmented control. Summary: sections on the left, **Email / Attendees / Your notes** cards on the right (stacks when narrow); Re-run ▾ menu above the sections; Edit shows on hover.
- **Meetings list**: toolbar search drives it (the duplicate field is gone); “Show all” clears a search; calmer rows.
- **Red for destructive actions** everywhere (two clicks: red text → red fill to confirm).
- Architect review — must-fixes applied: the brand no longer recolours the app's body text (`colors.text` is for documents only); the hex field uses the app typeface. Should-fixes: the pack's button styles (filled / soft / outline / ghost) really apply (data attributes on the content panel); the colour picker saves as you pick (debounced) and can't save its grey fallback; a replaced logo gets a new file name (no stale cached image, old file removed); the Email card is always there (ghost when no summary); tabs use `aria-selected` + a tabpanel; the ⋯ menu moves focus in, arrows navigate, Esc returns focus; nested unknown fields in tokens.json survive edits.
- Checks: 59 tests pass (new: ramp/hex validation, VIFC pack round-trips unknown fields); clippy clean; tsc clean; localhost-only passes. Screen Recording is now allowed, so I screenshot the app myself: the meeting page and the Meetings list are verified; the Workspace page and the Settings window still need a click from you (I can see windows but not click them).

## Manual steps
1. `pnpm tauri dev` is running with these changes. Sidebar → **Workspace** → Brand; Settings (⌘,) is now app-only.
2. Workspace → Brand: change Primary to another colour → the preview and the main window follow; pick a logo; Reset to starter.
3. Meeting page: ⋯ → Delete… is red; Rename by double-clicking the title.
4. Light/dark: in dark mode the brand ramp is used inverted and the canvas stays dark.

---

# Navigation: icon sidebar, back/forward, responsive layout
_2026-10-24_

## What changed
- **Sidebar, three states**: labels (240 pt) · **icons only** (76 pt rail with tooltips; workspace badge shows its logo mark, its switcher still opens full width) · hidden. Toggle with the **⟨⟨ Collapse / ⟩⟩** button at the bottom, View → **Icon-only Sidebar (⌃⌘S)**; ⌥⌘S still hides it. Until you choose, it follows the window: icons below 1000 pt wide, labels above. Your choice is remembered on this Mac.
- **Record** in the sidebar now starts a recording (Stop while one runs), like ⌘R.
- **Back / Forward**: ‹ › at the left of the toolbar, ⌘[ / ⌘], View → Back / Forward. Each greys out when there's nowhere to go.
- **Toolbar search works**: ⌘K focuses it, Enter opens Meetings with that search, Esc leaves it. It shrinks with the window; the breadcrumb hides when the toolbar is narrow.
- **Responsive content**: the content panel is a size container, so layouts follow the space they really have (sidebar open or not): field grids → one column, People rows → one column, Attendees editor → one card per person, Minutes/Summary/meeting-header/Models/Meeting-types rows wrap, tighter padding below 1000 pt.
- **Windows**: main window can shrink to 760 pt; Settings opens at 860 × 560 (min 600 × 420) and its tab strip scrolls sideways instead of overflowing (Welcome steps too).
- **Traffic lights** (previous message): `y` 18 → 28 in both windows, to centre them in the 52 pt bars.
- Architect review — must-fixes applied: the toolbar (and Settings/Welcome strips) drag the window again (`data-tauri-drag-region="deep"`; my toolbar rework had left only the edges draggable); after the welcome steps Back no longer returns to them. Should-fixes: record pill can't slide over search when the sidebar is pinned open in a narrow window; hiding the sidebar from the rail leaves no icons behind; picking the mode the width would choose returns to automatic; Settings window is resizable (so its responsive rules apply); back/forward state survives the welcome screen; ⌘[ / ⌘] are ignored while typing (no lost edits) and when there's nowhere to go; search follows `?q` and doesn't pile up history; focus rings, `aria-label`s, Record disabled while stopping, wide Markdown tables scroll. UI skill + D-029 updated.
- Checks: tsc clean; clippy clean; localhost-only passes; the debug build starts with the new menu.

## Manual steps
1. Quit fully, `pnpm tauri dev`.
2. Narrow the window below ~1000 pt → sidebar becomes icons; hover an icon → tooltip; widen → labels return. Click ⟨⟨ / ⟩⟩ → that choice sticks.
3. Open a meeting, switch tabs, open another → ‹ goes back through them, › forward; ⌘[ / ⌘] too.
4. ⌘K → type a name → Enter → Meetings filtered.
5. At 760 pt wide: Summary (Attendees → Edit), Minutes, Settings → People / Meeting types — nothing overflows sideways.
6. Light/dark and Reduce Transparency; traffic lights centred in the main window, with the sidebar hidden (⌥⌘S), and in Settings.

---

# Meeting types with their own templates + people memory
_2026-10-24_

## What changed
- **Settings → Meeting types** (new tab): pick a workspace; each type has a name, spoken language, *Default*, and its own minutes template (check report, Edit in Word, Replace…, Reveal in Finder, Reset to starter). *Add a meeting type* starts from a copy of the default type's template or a .docx you choose. Field reference at the bottom. VIFC gets **Weekly meeting** automatically from the template you gave (if you had replaced the workspace template, that one is moved in and a copy goes to the Trash).
- **Meeting page**: type picker next to the date; the Minutes tab says which template it will use. New Recording / Import Audio preselect the default type (its language too — Weekly meeting is Auto-detect; change it in Settings → Meeting types).
- **People memory**: Summary tab → **Attendees** (above the sections) → Edit: name, job title, organisation, attendance; add/remove; *Remember them for future meetings* (on). Save fixes the minutes + email now and teaches the memory; a corrected spelling is remembered as "also heard as". Every later summary is given the known people and corrected with them. **Settings → People** (new tab): find, edit, merge duplicates, forget, add.
- Migration **0003_person** (new `person` table; reviewed SAFE by migration-safety). Runs automatically on next launch for every workspace.
- **Crash fix (launch abort, 24 Oct 23:12)**: 0003 had already been applied to `vifc/workspace.db` at 22:48; adding a CHECK to the file afterwards changed its checksum, so sqlx refused to migrate and the setup error aborted the app. 0003 is restored byte-for-byte to the applied version (checksum verified; the JSON-array CHECK is dropped — Rust always writes a JSON array). Also: a workspace that fails to open at launch is now logged and left closed instead of stopping the app (`hub::open_all_active`). Verified: the debug build starts and stays up.
- Includes the earlier uncommitted text-model picker (D-027). The per-workspace "Minutes templates" section from that build is replaced by Meeting types.
- Architect review — must-fixes applied: correcting a wrongly named attendee no longer renames the remembered person (it's matched by the new name only; the model's spelling becomes "also heard as" only if it means nobody else); a People row resets after a merge, so a stale draft can't drop merged spellings. Should-fixes: only added/changed attendees teach the memory; names/titles/spellings are cleaned before they reach the prompt (no newlines or `<>`, length caps, ≤ 20 spellings); a spelling another person already has is refused (merge instead); the prompt says known people are reference only (no invented attendees); merge and type delete are transactions; no orphan template if creating a type fails; a rename can't be undone by a language change; the type picker shows errors; the Minutes tab names the template export will really use.
- Checks: 57 tests (new: Vietnamese name folding, remember/correct/merge incl. the wrong-attendee case, prompt cleaning, type-file path guard); clippy clean; tsc clean; localhost-only passes. Not yet run in the app.

## Manual steps
1. `pnpm tauri dev` (first run rebuilds ~10 min; migration 0003 applies).
2. Settings → Meeting types → VIFC: *Weekly meeting · Default* with ~N fields, all recognised. Add "Partner call" with your partner-call .docx (or a copy) → Edit in Word.
3. Open a summarised meeting → set its type in the header → Minutes tab says *Template: Partner call* → Export → right template.
4. Summary tab → Attendees → Edit → fix a name/title/organisation → Save (Remember on). Settings → People shows them with the old spelling under "also heard as".
5. Re-run the summary (or summarise another meeting with the same people): the attendees come out corrected.

---

# Text model picker + minutes template management
_2026-10-24_

## What changed
- **Settings → Models → Text model** (new, top): every model in Ollama with its size; the one in use is marked *Writes summaries*. Per model: **Use for summaries**, **Update** (re-download), **Remove** (not the one in use). **Download another model** by name (e.g. `qwen3.5:4b`), with progress. **Free memory now**. Opening Ollama when it isn't running. The choice is saved (app setting `ollama_model`, default `qwen3.5:9b`) and applies to the next summary; a summary already running finishes with its own model.
- **Settings → Workspaces → Minutes templates** (new, under the workspace list): per workspace *Starter template* or *Custom · changed <date>* plus a check (fields found, unknown fields that would come out empty, fields not used). **Edit in Word** (the starter is copied into the workspace first), **Replace…** (pick a .docx → check report → *Use this template*; refused if it can't be filled), **Reveal in Finder**, **Reset to starter** (custom one → Trash). *Fields a template can use* lists every `{{field}}` and list block.
- Backend: `ollama::{model, select, remove, valid_name}`, pull by name, `TextSession` pins its model; `modules/meetings/template.rs`; `docx::placeholders`; `model::{sample, fields}`. Commands: `select_ollama_model`, `remove_ollama_model`, `unload_ollama_model`, `pull_ollama_model(model?)`, `get/check/replace/reset/open_minutes_template`, `minutes_template_fields`. No migration. D-027.
- Architect review — must-fixes applied: switching the model waits for a running summary, and each summary keeps (and records) the model it started with. Should-fixes: the model in use can't be removed (backend too); asking for a second download while one runs says so; the template check only opens a picked `.docx`; Replace checks the exact copy it installs; starter copies are atomic; confirm buttons disarm after 4 s; stale errors cleared.
- Checks: 54 tests (new: model-name validation, VIFC starter passes the check with no unknown fields, junk file rejected, field list); clippy clean; tsc clean; localhost-only passes. Not run in the app yet (debug build cache was cleared; first `pnpm tauri dev` rebuilds, ~10 min).

## Manual steps
1. `pnpm tauri dev` → Settings (⌘,) → Models: both `qwen3.5:4b` and `qwen3.5:9b` should be listed (they're in your Ollama). Click *Use for summaries* on 4b → Re-run a summary → the Summary tab's footer shows qwen3.5:4b. Switch back to 9b.
2. Settings → Workspaces → Minutes templates → VIFC shows *Starter template · N fields · all recognised*. *Edit in Word* → change a heading, save → the row now says *Custom* → export minutes → your change is there.
3. *Replace…* with any ordinary .docx → the report says it uses no fields (all unused) → Cancel. *Reset to starter* → custom file in the Trash.

---

# P4.1 (icon, notifications, onboarding, backup, New Dawn.app)
_2026-10-24_

## P4.1
- **Icon**: sunrise, `src-tauri/icons/AppIcon.icon` (Icon Composer, Liquid Glass) → `Assets.car` + `CFBundleIconName`; flat twin `AppIcon-flat.svg` → `icon.icns`/PNGs for macOS 14–15. Rebuild with `./scripts/build-icon.sh` (needs Xcode; first time `xcodebuild -runFirstLaunch`, already done on this Mac).
- **Notifications**: Recording started / Recording stopped (title · length — transcribing) added; one helper `kernel/notify.rs`.
- **Welcome** (first launch, every step skippable; New Dawn → Run Setup… brings it back, ignored while recording): Permissions (mic, call audio explained, test notification) → Text model (Ollama status, Open Ollama, Download qwen3.5:9b with progress via localhost `/api/pull`) → Speech model → PDF → Workspace → Done. **Your current install will show it once on next launch** — Skip setup is fine.
- **Backup** (Settings → General → Backup): choose a folder → daily `New Dawn yyyy-mm-dd/` with registry.db, every workspace.db and each workspace's brand folder; newest 7 kept; Back up now; last backup / error shown. Off until a folder is chosen. Recordings and exports not included.
- **Logs**: `~/Library/Logs/co.majorisdigital.newdawn/new-dawn.<date>.log`, 7 days.
- npm `@tauri-apps/api` ~2.11, `plugin-opener` ~2.5, `plugin-dialog` ~2.7 pinned to the Rust crates (the build refused the mismatch). Crate added: `tracing-appender`.
- Architect review — must-fix applied: a malformed model answer no longer puts meeting text in the error (now in the log files). Should-fixes applied: backup includes the brand folder and an archived workspace's WAL; one backup at a time, partial folders always removed (and leftovers pruned); a failed prune no longer fails the backup; Settings shows the plain error; Run Setup… ignored while recording; returning to the Qwen download reconnects to it; pull flag reset even on panic; status changes always reported; logs flushed at quit; onboarding flag moved into the hub.
- Checks: 52 tests pass; clippy clean; `pnpm tsc` clean; localhost-only passes.

## Build — exact command
```
cd new-dawn && pnpm install && pnpm tauri build
```
Output: `src-tauri/target/release/bundle/macos/New Dawn.app` and `src-tauri/target/release/bundle/dmg/New Dawn_0.1.0_aarch64.dmg` (Apple Silicon). Ad-hoc signed (`signingIdentity: "-"`), hardened runtime, entitlements: microphone, Apple Events. Not notarised (no Developer ID): on another Mac, first launch = right-click → Open. Needs ~10 GB free for the release build cache.

## Manual steps — the P4.1 done-check (fresh macOS user account)
1. Copy the `.dmg` to the other account (e.g. /Users/Shared), open it, drag New Dawn to Applications, right-click → Open the first time.
2. Welcome: allow the microphone, send a test notification (allow), open Ollama (install it first from ollama.com) → Download qwen3.5:9b, download Whisper turbo, check converters, create a workspace → Open New Dawn.
3. ⌘R → record 1–2 minutes of a call (allow call audio when macOS asks) → Stop: notifications for stopped, transcript ready, summary ready.
4. Settings → General → Backup → Choose folder… → Back up now: a `New Dawn <date>` folder appears.
5. Dock and Finder show the sunrise icon (Liquid Glass on macOS 26).

---

# P3.3 (review, edit, re-run, history)
_2026-10-24_

## P3.3
- **Summary tab**: *Your notes* (autosaved, read by the next run); per-section **Edited** chip; editing hints per section ("- **Topic**" + "  - point", "Owner — task — due"…); **Re-run / Deep re-run** ask *Keep my edits* (default) or *Discard edits* when something was edited; "transcript was corrected after this summary" notice.
- **Edits reach the minutes**: edited Key points / Decisions / Action items / Open questions rebuild the lists the DOCX is filled from (details like "decided by" kept when the text matches).
- **Transcript tab**: Edit on each line (hover or Tab to it) → Enter saves, Esc cancels; search follows.
- **Meeting header**: Rename (email subject follows), Delete… (sheet; recording → Trash, exported files kept).
- **Minutes tab**: Delete per export (two-step, files → Trash); hint now says the minutes include your edits.
- Backend: `modules/meetings/review.rs`; commands `update_transcript_segment`, `set_meeting_notes`, `rename_meeting`, `delete_meeting`, `delete_minutes_export`; `summarise_meeting(deep, keep_edits)`; `Meeting.notes_md`, `Section.edited`, `SummaryDoc.generated_at`, `SummaryView.transcript_changed`. No migration. D-025.
- Architect review — must-fix applied: a summary edited before P3.3 (no per-section flags) now asks before Re-run replaces it ("This summary has older edits"). Should-fixes applied: open section editors survive saves elsewhere; decision/question details kept even when the old text had Markdown; `generated_at` stamped before the transcript is read (corrections during a run are flagged); notes saved on blur and re-read by the run; notes given to every part of a long meeting; the subject uses the title as of the end of the run; "Transcribe again" asks (it replaces corrections); an open line editor closes while transcribing; Enter ignored during Vietnamese IME composition; Delete sheet focuses Cancel; "Minh - task - Friday" with plain hyphens parses; trash only `recordings/<id>` (single path component) or a canonical file under `recordings/`, off the async workers; delete refused while a job is registered; the notes heading can't be spoofed from the transcript; "Not discussed" without a period is a placeholder.
- Checks: 50 tests pass (new: bullet parsing, edits → lists, re-run keeps edits, segment correction + FTS, notes, rename, delete — the delete test never touches the real Trash); clippy clean; `pnpm tsc` clean; localhost-only passes; app rebuilt and running.

## Manual steps — the P3.3 done-check (a messy real meeting, one sitting)
1. Transcript tab: fix a few misheard names (Edit → Enter).
2. Summary tab: add notes (e.g. "Chair: …, Acme attendees: …"); the stale-transcript notice shows → **Re-run** (no edits yet, so it just runs).
3. Edit Decisions and Action items in the given format → Save → chips show *Edited*.
4. **Re-run** again → choose *Keep my edits*: your sections stay, the others are rewritten.
5. Rename the meeting → Minutes tab → Export (Word, optionally PDF) → Open: title, decisions and actions match your edits.
6. Delete an old export (Delete → Move to Trash?), and delete a throwaway test meeting (Delete… → recording appears in the Trash).
7. Light/dark and Reduce Transparency on the three tabs.

---

# P3.2 stages B + C (minutes export, PDF on request, Minutes tab)
_2026-10-24_

## P3.2 stage B (engine)
- **Summary JSON extended** (one call): attendees, discussion per topic, decisions with decided_by/condition, open issues with who/by, next_focus. Email Key points now grouped by topic. Old summaries still load. Live Qwen run: 42 s, all lists filled correctly.
- `kernel/export/model.rs` (template fields, local-time date line, `BB-DDMMYYYY-NN`), `kernel/export/pdf.rs` (Word → LibreOffice → Pages, 60 s, timeouts say "is it showing a dialog?"), `modules/meetings/minutes.rs` (export orchestration, `document_export` rows, never overwrites, template = workspace's own or bundled pack by slug).
- Commands: `export_minutes_docx`, `export_minutes_pdf` (converts the DOCX as saved, incl. Word edits), `list_minutes_exports`, `list_converters`, `open_minutes_file` (open/reveal by export id — no raw paths from the UI).
- Template: numbering moved into the template (`3.{{n}}`, `QĐ-{{nn}}`, `CV-{{nn}}`); regenerated.
- Crates: `chrono` (clock), tokio `process`. Info.plist `NSAppleEventsUsageDescription`; entitlement `automation.apple-events`. No migration.

## P3.2 stage C (UI)
- **Minutes tab**: card with Word / Word + PDF (Word by default — PDF only when chosen), file name (placeholder = default name), Export; hints for template [ ] fields and for Summary-tab edits; **Exports** list (newest first: Open, Open PDF or Make PDF, Reveal in Finder; ghost cards when empty); **PDF preview** of the newest PDF (asset protocol; scope `exports/**`, CSP `frame-src`).
- **Settings → Converters**: Word / LibreOffice / Pages with Found / Not installed and the path. `GhostTab.tsx` removed (no longer used).
- Checks: 45 tests pass (+ live: Word conversion 10.5 s, Qwen summary 42 s); clippy clean; `pnpm tsc` clean; localhost-only passes; app launches without errors.
- Architect review — must-fix applied: PDF conversions run one at a time (process-wide lock) and the Word script picks its document by a unique file name, never "the active document" (two jobs could otherwise save/close each other's file or yours). Should-fixes applied: stored export paths must resolve inside the workspace's `exports/` with the right extension before opening/converting; file names capped at 200 bytes and a typed `.docx` stripped; the name is reserved with create-new (no overwrite race) and removed if the export fails; **Update PDF** re-converts after Word edits; `list_converters` async; a decision without `text` no longer fails the summary; open/reveal moved into the module; CSS tokens; Converters shows load errors.
- Not yet re-verified live: the changed Word script (Word was blocked by a dialog on screen — AppleEvent timeouts).
- `cargo clean` freed 28 GB (disk was full); first rebuild is slower.

## Manual steps — the P3.2 done-check
1. `pnpm tauri dev`. Open a summarised VIFC meeting → re-run the summary once (**Re-run**) so it has the new attendees/discussion lists.
2. Minutes tab → **Word** → Export → **Open**: it should look exactly like the template, filled in.
3. **Make PDF** on that row (or export with Word + PDF). First time, macOS asks whether New Dawn may control Microsoft Word — allow. If it also asks to access data from other apps, allow (that's Word's own folder).
4. The PDF preview shows under the list; **Reveal in Finder** opens the exports folder.
5. Light/dark and Reduce Transparency on the Minutes tab and Settings → Converters.
6. If a PDF times out: look for a dialog in Word, close it, try again.

---

# P3.2 stage A (DOCX template engine + VIFC template)
_2026-10-24_

## P3.2 stage A
- `src-tauri/src/kernel/export/docx.rs` — template engine (fields, nested blocks over paragraphs/rows/ranges, run merging, byte-exact round trip, atomic zip write). 14 tests incl. split placeholders, 3 action rows, grouped rows in one table, nested discussion blocks, a block inside a table cell, and filling the real VIFC template.
- `scripts/vifc_template_placeholders.py` → `brands/vifc/templates/meeting-minutes/template.docx` (original kept as `original.docx`); VIFC logo SVGs copied into `brands/vifc/logo/`.
- Crates: `zip` 9 (deflate only), `quick-xml` 0.42. The module carries a temporary `expect(dead_code)` until stage B wires the commands.
- Visual check: sample filled → Word → PDF → rendered pages; 4 defects found and fixed (run-level formatting in agenda/bullets/open issues; the engine repeated a whole table for grouped rows). Re-checked after the review fixes: 3 pages, identical layout.
- Architect review — must-fixes applied: a block marker split across runs in a cell no longer recurses forever (runs merged before blocks; marker stripping errors instead of looping); an empty row-level block leaves no cell without a paragraph and no table without rows; a UTF-8 BOM is stripped before parsing. Should-fixes applied: attendees use `job_title` (no clash with `title`); control characters dropped and `\r` → line break; untouched runs beside a placeholder stay byte-identical; the temp file is removed on error; marker paragraphs removed only when empty; strings/numbers open optional blocks. `docs/document-export.md` §2.2 now documents the real syntax.
- Side effect: `cargo fmt` reformatted `lib.rs` and `modules/meetings/mod.rs` (formatting only).
- D-023. Tests: 38 pass; clippy clean; localhost-only check passes (OOXML `xmlns` identifiers allowed).

## Manual check (optional now, required at the end of P3.2)
- Open the sample in Word: `/private/tmp/claude-501/-Users-tienvnguyen-Astral-Archive-SEED-new-dawn/085754d7-6ba5-452b-9e4b-235e4d6e7e44/scratchpad/sample-minutes.docx` (PDF next to it: `…/scratchpad/sample-minutes.pdf`).
- Open `brands/vifc/templates/meeting-minutes/template.docx` in Word to see where the `{{fields}}` sit; edit wording freely in Word — keep the `{{…}}` markers intact.

---

# P3.1 stage B (Summary tab)
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
