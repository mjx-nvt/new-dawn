# Architectural Decisions

Newest first. Every entry: what was decided, why, and what it means for future work. Keep entries short; link to `docs/` for the long version.

## D-043 — Live 0.7.5: you and the call apart, how clearly each line was heard, both ways, a quick translation that keeps nothing; task priority
_2026-10-26 · local build, not released_

**Decision** (user: "live translation should have one presenter or discussion — if 2 people talk at the same time and the app can't separate the transcript, the translation would be badly wrong"; "an option to save or not — sometimes I just need a quick translation when people switch between EN and VI"; "task items should have a priority"; "remove red, add green, or − and +"):
- **No speaker-recognition model (diarization) yet.** The local options either need Python/ONNX stacks or a third model in memory beside Whisper and Qwen on a 16 GB Mac, are English-first, and none separates two people talking *at once* — they only label turns. Deferred.
- **The audio already separates people**: the recorder captures the microphone (you) and the call's sound (everyone else) apart. In a **Discussion** each is cut into lines on its own (two segmenters sharing one VAD), so talking over each other gives two clean lines instead of one garbled one. A line of yours waits until the call is settled 600 ms past its end (at most 16 waiting) so lines appear in speaking order. A microphone line that only caught the call through the speakers (≥ 50 % of its syllable pairs said by the call within ±0.9 s) is dropped and counted; a line under 4 words is compared only with call lines of about its length — a question holds its own answer ("Anh có đồng ý … không?" → "Không."), a copy doesn't. From 3 drops in the last minute the page says to use headphones (and stops once they're on). Labels **You** / **Them**; "You" appears once the call has spoken (no call audio = no labels). **Transcripts read by time** (`start_ms`, then `seq`): a line of yours that waited for the call's quick reply keeps the earlier number in arrival order but reads first everywhere — the Live page, the saved transcript, the summary and the task finder.
- **One presenter**: one mixed stream, longer lines (800 ms pauses, up to 15 s), read as paragraphs; the translator is told consecutive lines continue one talk.
- **How clearly, instead of a confident wrong translation**: live decoding no longer retries at higher temperatures (the retries invent text) and stops at 220 tokens. Each line gets a score — the mean over its words of each word's least likely token. **Unclear** (score < 0.50, average log-probability < −1, the token cap hit, or repeats collapsed): shown, marked, *not translated*. **Uncertain** (< 0.70): translated literally ("[…]" for what can't be made out) and marked "May be inaccurate". Thresholds are first guesses to calibrate on real calls (clear test lines scored 0.80–0.86).
- **Languages**: Tiếng Việt → English (default), English → Tiếng Việt, or **Both ways** — each line is decoded in its voice's current language; when it comes out unsure, on the voice's first line, or when it doesn't look like that language (a line of 3+ words heard as Vietnamese with no Vietnamese letter or tone mark, or heard as English with one — Whisper forced to Vietnamese writes clear English as English, sure of it), Whisper's language detection between the two decides, the line is decoded again if needed, and the voice takes that language once the line is kept (a dropped echo changes nothing). The meeting's language is vi / en / auto.
- **Quick translation** (`save: false`): no meeting, no folder, no recording; lines live in memory (the last 2,000) and stay readable and copyable until the next recording starts or the app quits. The backend keeps the session's last status with them (`last_quick`), so the Live page shows an ended session after Stop from any page, after a window reload, and says when it stopped on its own; they're cleared only once a new recording has really started. Copy all goes through the clipboard plugin (like Copy as email): the webview's own clipboard isn't there in every build. The AI is still reserved for the session.
- **Saved meetings keep who and how clearly** (migration 0010, additive: `transcript_segment.speaker / clarity / lang`, `task.priority`): the summary, the task finder and its evidence see "{owner}:" and "Call:" before labelled lines.
- **Weekdays and ordinals are corrected in code** (`kernel/ai/days.rs`): the fast model turned "thứ sáu" into Saturday (8 of 8), "Friday" into "thứ Năm" and "Thứ hai là…" into "First is…" — Vietnamese counts days from Sunday. A guide rule fixed weekdays but broke ordinals, so the number is now set from the line that was said whenever both sides name as many: mentions pair up by number over every order (English moves the day to the end, "thứ sáu thứ ba" is "the third Friday"), only one-apart slips are corrected (a different day is left alone), and "mọi thứ năm nay" (everything this year) or "thứ mười hai" (12th) aren't mentions. The model's choice of weekday vs ordinal is kept.
- **Keeping up** (found in the running app, where the live text skipped 27 s in the first minute): the Silero VAD runs on **one thread** — its steps are a 32 ms frame each, and 8 threads only wait for each other (10 s of audio: 20 ms on 1 thread, 114 ms on 8; live runs it on every 600 ms window for each voice while Whisper and Qwen use the cores; file imports gain too). And a finished line no longer waits for its translation until the talk pauses: once it has waited 6 s it is translated before the next utterance, unless 4 or more utterances are waiting (then transcription keeps going first, so no audio is skipped).
- **The translator asks Ollama for an 8k context** (the app's 32k is for summaries). Note: Ollama on this Mac runs Qwen through MLX; the 4B starts near 4 GB and grows with use (5.7 GB after ~50 lines, 8.6 GB seen after ~80) — beside Whisper large-v3 that means swap on 16 GB in long sessions.
- **Task priority** as in Apple Reminders (none / ! / !! / !!!), never red (red = overdue): Today and No date sort by it, Upcoming by day then priority.
- **List controls**: in All people, a red ⊖ takes someone out of the attendees and a green ⊕ adds them — Apple's list-editing controls, the one exception to "red = record/destructive". Models are one row each with the row's action and a ⋯ menu (Update, Remove).
- Out of scope: diarization within one stream, renaming "Them", switching format mid-session, echo cancellation, a floating subtitle window.

---

## D-042 — Tasks: your to-dos from meetings, as simple as Apple Reminders
_2026-10-26_

**Decision** (user: "a Tasks tab right below Live; whatever action item is related to me, the AI recognises it and creates a task; every action item gets an Add-to-task button; keep it simple like Apple Reminders or Microsoft To Do"):
- **One list per workspace** (like Meetings): Today (incl. overdue), Upcoming, No date, Completed (collapsed). Add a task by typing; a due chip (Today, Tomorrow, Next week, Pick a date, Remove); tick, edit in place, delete. Left out of v1: several lists, an all-workspaces view, notes, steps, repeat, times and reminders, tags, search.
- **Which items are yours** (`modules/meetings/tasks.rs`), once per meeting after its first summary: rules first — the item's owner is you by name, alias, first/last name or your title here, and isn't another attendee too → a task; every owner is someone else → none; the rest (Unassigned, groups, unknown names, items that name you) → one call to the **Tasks agent** inside the summary session (thinking off; your notes and the transcript lines that bear on each item; an answer of item numbers with a reason, plus a `not_mine` list, which stopped it claiming team items given to others — 12 of 12 live runs right). Nothing happens until your name is set.
- **Identity**: a task links to its action item by `task_id` stored on the item (renames keep it; re-runs and edits carry it with the same rule as ticks). A deleted task is a tombstone, so it never comes back automatically; "Add to Tasks" restores it. `UNIQUE (meeting_id, source_key)` guards double clicks. Re-runs never auto-add again (`meeting.tasks_at`); the button covers new items. No cross-meeting dedupe in v1 — a visible duplicate beats a hidden task. Migration 0009 (additive: `task`, `meeting.tasks_at`).
- **Ticks sync both ways** with the summary (a tick is not an edit). Tasks from others' items read "Minh: …".
- **Due dates** from what was said, counted from the meeting's local day, no AI needed: English and Vietnamese weekdays, today/tomorrow/ngày mai, next week/tuần sau (= that Friday, a deadline), end of month/cuối tháng, explicit dates (day first), "in 2 weeks"; ordinary words that look like dates (thu, mai) only count next to a dating word. Unparsed phrases stay as written; the item's own text is never changed.

---

## D-041 — Live: Vietnamese captions with the English beside them, as people speak
_2026-10-26_

**Decision** (user: "live transcription and translation, a tab right below Meetings; learn from Transync AI; a very simple version first: live transcript VI and translate to EN"):
- **A live session is a recording** (`live: true`): the same recorder, pill, banner, ⌘R/tray Stop and crash-safe WAV; it becomes a normal meeting in Vietnamese. No second capture path.
- **Pipeline** (`kernel/ai/live.rs`, the only new AI path): the mixer offers each 600 ms 16 kHz window over a **bounded** channel (`try_send`; drops counted and shown). A segmenter thread cuts utterances with Silero VAD — after ≥ 600 ms of silence or at 10 s (cut at the least speech-like 100 ms), hysteresis 0.5 / 0.35, 300 ms kept either side, shorter than 250 ms dropped. The AI loop transcribes each utterance (Vietnamese forced, greedy, one segment, the vocabulary prompt with the owner, the echo filter and the corrections) and translates each finished line once (**Live translation agent**, thinking off, temperature 0.2, the last 3 pairs as context, the people/terms list as glossary). Transcription comes first; a shown line is never rewritten.
- **The one exception to "one model at a time"**: the session holds the AI queue from Start to Stop, with Whisper and the translation model both loaded. Translation defaults to the fast tier `qwen3.5:4b` (large-v3 + 4b ≈ 7–8 GB on the 16 GB M1 Pro; 9b next to Whisper risks the grey-window kills of D-034). Start refuses while another AI job runs; batch jobs wait for Stop. Without Ollama it runs Vietnamese only. If live fails mid-session, a notification says so and Stop transcribes the recording the normal way.
- **Saved as it goes**: lines into `transcript_segment` with the English in `translation` (migration 0008, additive; search still indexes the Vietnamese). Stop doesn't re-transcribe: the meeting becomes transcribed, the WAV is compressed and the automatic summary runs. Transcribe again replaces the live lines and their English (its confirm says so); editing a Vietnamese line keeps its English.
- **UI**: Live under Meetings (File → New Live Translation…, ⇧⌘R); Tiếng Việt | English columns, "Translating…", one status word (Listening / Transcribing / Translating / Behind by N / Vietnamese only), A−/A+ text size, follow-the-latest with a "↓ Latest" capsule, the AI marker "Translated on this Mac by {model}". The meeting page shows both columns afterwards.
- **Measured** on the 13 s test recording: Vietnamese ≈ 2 s and English ≈ 3–4 s after each pause, from a cold start. Not word by word in v1.
- Learned from Transync's public pages only — its terms forbid reverse engineering, and it runs in the cloud. Later, in order of value: a floating subtitle window, word-by-word partial text, "you / them" labels, two-way, translation-only view.
- Supersedes D-007's "no live transcription" and, for live sessions only, "never two models at once".

---

## D-040 — A brand from a logo, a slogan and a colour; the minutes template is made from it
_2026-10-26_

**Decision** (user: "simplify the brand: in the wizard I give the original logo → the white version is made; a slogan in EN → AI translates to VI; the primary colour — that's all. Then create a default DOCX template for me, learning from the uploaded template"):
- **White logo, made exactly** (`kernel/brand/logo.rs`): SVG → every fill, stroke and stop colour white, opacity kept (the official VIFC white SVG is reproduced pixel for pixel); PNG → white with its alpha kept, or the background removed first when the PNG is opaque. Written whenever the main logo is set (`logo/logo-white-*`, `logo.on_dark`); a new or removed logo deletes only a white version the app generated, never an official one or one the user picked. An SVG logo can't open any file it names (`<image href>` to local paths is refused; embedded `data:` images still draw). New dependency **resvg 0.47** (pure Rust, offline; `rust-version` 1.87) to rasterise SVG logos for Word.
- **Translation agent** (`Agent::Translate`, guide `agents/translate.md`, model picker in Workspace → Agents): one Qwen call, thinking off, EN → VI, answer cleaned to one line. Translate buttons on the Brand page and in the wizard (which translates once by itself when a new English slogan is left with no Vietnamese).
- **A document palette from one colour** (`tokens::document_palette`): accent (darkened until ≥ 4.5:1 on white), deep (≥ 7:1 — titles, labels, header-row fills), four tints (lightest fills → VI labels on the deep fill, ≥ 4.5:1 there). Fitted so `#0132AD` gives back the VIFC template's own colours.
- **The template** — the app makes it; no model is needed, only the slogan uses AI. `scripts/make_minutes_base.py` derives a neutral base (`src-tauri/templates/minutes-base.docx`, compiled in) from the uploaded VIFC template: generic eyebrow, the VIFC-only rows and the footer pattern removed, slogans and the workspace name as placeholders, the minute-taker filled. `kernel/export/rebrand.rs` recolours it in one pass (the six VIFC blues → palette roles; slate neutrals stay), applies the brand's heading and body fonts, puts the logo in the header (PNG at its true aspect, capped to the cell) or the workspace name when there is no logo, and names the workspace as the file's author. It becomes the default meeting type's template in the wizard, with **Update from brand** on the Brand page, with **Reset template** in Meeting types (a starter pack still wins there), and automatically for any workspace that has no template at all (before, every workspace except VIFC had none and export failed).
- **Preview** of the real result: the template filled with sample data and the real workspace, slogans and owner, drawn by macOS Quick Look (`qlmanage`, 20 s timeout) on the Brand page and in the wizard.
- **The wizard (`/setup`) is real**: Workspace (name, slug, your title — and your name if the profile has none) → Brand (logo with its white version on the brand colour, slogans with Translate, colour with its 11 shades) → Minutes template (preview, check, Open in Word, Make again) → Finish. The mock meeting-types and review steps and the fake document preview are gone; the Brand page dropped Secondary / Accent / Text (nothing read them). Supersedes D-009 (brand import from DOCX/PDF, never built).
- **Template fields** added: `workspace_name`, `slogan_en`, `slogan_vi`, `minute_taker` ("Name — Title"), `minute_taker_name`; the VIFC starter fills the minute-taker too, and keeps its own header text (so VIFC's slogan edits don't reach its minutes — the Brand page says so). The default heading font is Playfair Display, from the uploaded template.

---

## D-039 — All people and the attendees; the owner's profile
_2026-10-26_

**Decision** (user: "this is a non-bot meeting AI, so the app doesn't know who joined: split the box into all people and attendees — only attendees go into the minutes — with an option, default for all meetings, to add the people involved"; "my profile in App settings, name only for now; each workspace inherits my name with a different title"):
- `doc.attendees` keeps its JSON key and now holds **everyone the meeting involves** (who speaks, presents, is addressed, decides, gets a task, or is discussed as a participant). `Attendee.attending` marks the **attendees** — the only people in the minutes. Summaries saved before have no flag and read as attending, so their minutes don't change. The card **All people** has **Attendees** (Remove) and **Also involved** (Add, Add all); ticking someone never teaches the people memory and never rewrites the prose or the email.
- App setting **Everyone involved attends** (Settings → General, on by default) sets the flag for new summaries; off, only the owner and the presenter start as attendees. A re-run keeps the owner's choices — by exact name, then by the same person: every name word of one in the other, and at least one of them not a common family name or Văn/Thị, so "Mr. Nguyen" is never taken for "Nguyễn Văn Tiến". Old names are first spelled as the people memory knows them now; an attendee the new run didn't list comes back only while the transcript or notes still mention them (a hand-added person nobody mentions does not — ponytail: an "owner set this" flag would fix that).
- **Profile**: the name is app-wide (registry setting `owner_name`); each workspace adds the owner's **title** (registry migration 0002, `workspace.owner_title` — the first registry migration; older builds can't open the registry once it has run, and the user chose not to keep compatibility with past data). The owner is the first attendee of every new summary (with that workspace's title and name), the **minute-taker** in the minutes, signs the email (Markdown hard line breaks, so Copy and Open in Mail keep the lines), is named in the summary's fixed rules (never in the owner-notes block — that would trigger highlights), leads the Whisper prompt's people and is never a naming doubt. `keep_mentioned` keeps the owner and the presenter even when nobody says their name.
- Fixed on the way: a long meeting's merge step showed the model `"done": false`; an echoed flag made the whole summary fail to parse. Flags (`done`, `attending`) now stay booleans.

---

## D-038 — One to-do list per meeting; meeting format and presenter; a model per agent; people checked against the transcript
_2026-10-25_

**Decision** (user: "action items and next steps are the same — combine them and go through every person so no task is missed"; "some meetings have one presenter whose name is never said — let me enter it at upload or right after recording"; "assign a model to each agent"):
- **Action items is the to-do list**; there is no separate Next steps section in new summaries (older ones keep theirs, shown as tasks). The model is told to go through every person named — each attendee, everyone mentioned, the presenter — and list every task, commitment, request and follow-up, then unowned next steps as "Unassigned". Tasks carry `done` ("- [ ] / - [x]"), survive edits and re-runs, and show as Done/Open in the minutes. Under the list, **"No task for …"** names present attendees who own none — a cheap coverage check.
- **People are checked against the transcript** (`keep_mentioned`): a live test on a 13-second clip showed the model listing 13 attendees from the people memory it is given for spelling. Attendees must be mentioned (any name word of ≥3 letters, titles ignored, people-memory spellings included) in the transcript, the notes or the presenter field; tasks owned by someone never mentioned become "Unassigned".
- **Meeting format** (migration 0007: `meeting.format`, `meeting.presenter`): Discussion or One presenter (+ name), set in Import audio, Start recording, or on the meeting page any time. Whisper's prompt names the presenter; the summary is told who to attribute statements to (presentation: unattributed = presenter, others = Q&A; discussion: attribute every position and cover every participant); the presenter is listed as an attendee.
- **A model per agent** (workspace setting `agent_model.<id>`): Summary, Email (Rewrite), Corrections each pick an installed model or follow the app default; the automatic run (summary → names check → cover note) stays on the Summary agent's model so Ollama doesn't swap models mid-run. Each agent card shows runs and average time per model, so the choice can be made on evidence. Still one text model in memory: a session on another model unloads the previous one, and Whisper unloads both the app's and the last-used model. An agent whose model was removed falls back to the app's model (logged) instead of failing.
- **A tick isn't an edit**: ticking tasks leaves the section unedited (a re-run still writes it fresh) and doesn't mark the email as older; ticks carry over to the same task (owner + task) after a re-run — with or without "keep my edits" — and after a correction. A kept, edited old Next steps section survives a re-run at the end of the summary.
- A discussion has no presenter (switching back clears it) and adds no format note to the prompt (it would count as owner notes).

---

## D-037 — A corrections agent instead of name lists in the notes; detailed summaries; next steps as tasks
_2026-10-25_

**Decision** (user: "Transcribe again doesn't work; re-run with notes isn't smart and seems token-limited; next steps should be a to-do list; instead of putting names in my notes I want a box that updates the summary *and* the transcript without a full re-run"):
- **Transcribe again** accepted only Recorded / Failed / Transcribed meetings, so a summarised meeting was refused. It now accepts Summarised too (then re-summarises, keeping edits). Verified live on the test meeting.
- **Summary**: the answer budget goes from 6k to 14k tokens (18k deep) — the transcript part then gets ~16k before being summarised in parts; the instructions and the default summary guide ask for detail (3–8 sentences per topic, who/what/figures/reasons). A lenient JSON reader (`summarise::lenient`) turns nulls, numbers and stray objects into the expected text, after a notes re-run failed on a type mismatch. **Next steps** are `- [ ]` tasks, shown as a tickable list (ticking saves the section; older bullet lists show as tasks too); ☐/☑ in exported documents.
- **Corrections agent** (`modules/meetings/corrections.rs`, guide `agents/corrections.md`): the owner writes what is wrong; one Qwen call returns exact replacements, people (title, organisation, wrong spellings) and, for facts, rewritten sections. Applied to the summary (renamer, any case), the email, and the **transcript** (exact spelling, three letters or more — the pairs are written by the model, so the D-036 rule holds; Whisper's text kept for Undo), then the people memory learns the people and the vocabulary learns only *respellings* (a corrected fact such as "15 Oct → 10 Oct" fixes this meeting, never future transcripts), and pending name suspects it resolves disappear. The Corrections card sits under Attendees; Notes stay for what to highlight.

---

## D-036 — Corrections steer Whisper (vocabulary); a fixed two-column summary; progress with time left
_2026-10-25_

**Decision** (user: "is there any way the adjustments I make could feed back to Whisper for a better transcript later on — I really need this"; "notes on top, pinned; email and attendees scroll"; "a progress indicator so I know how much time is needed"):
- **Whisper can't be retrained here; it can be steered.** Each transcription gets an *initial prompt* (whisper.cpp `initial_prompt`, ~220 tokens): the meeting title, the people memory's names (recent first) and the workspace vocabulary's terms — this biases spelling. Right after Whisper, **known mishearings are replaced** (vocabulary mishearing → term, people "also heard as" → name; whole words, the same renamer as the summary; aliases under three letters skipped). What Whisper wrote is kept per line (`transcript_segment.raw_text`) so each correction shows "Corrected" with **Undo**.
- **Learning is deterministic**: a transcript line edit is diffed word by word (LCS); a changed run of ≤4 words becomes mishearing → term only if the new run looks like a name or term (capital, digit or non-ASCII letter), so wording edits aren't learned. Name confirmations and attendee edits already teach the people memory, which feeds both the prompt and the replacements. Summary prose edits are not learned (too ambiguous).
- **Workspace → Vocabulary** shows what Whisper is told, lists terms (Learned / Added) with their mishearings, add / edit / remove. Migration 0006 (additive: `vocabulary` table, `transcript_segment.raw_text`).
- **Layout**: the main window is at least 1180 pt; the Summary tab is always two columns (400 pt aside); Notes first and sticky; the auto icon rail starts below 1280 pt. WebKit insets sticky offsets by the scroller's padding, so pinned elements use `top: calc(-1 * var(--space-7))` (verified by screenshot).
- **Progress**: one bar with elapsed and "about N min left" for transcription (from Whisper's percentage) and for summary / naming / email (live token rate against the agent's typical output from `ai_log`, or its typical duration before the first token; elapsed only with no history).

---

## D-035 — Agents with editable guides; highlights from the owner's notes; the naming step asks
_2026-10-25_

**Decision** (user: "each task should have an agent with a detailed guide, so I can improve the agents; my notes should highlight what matters; naming corrections should ask me to confirm; the email should be a cover note with detailed key points"):
- **Four agents** — Summary, Naming, Email, Minutes — each with a Markdown **guide** in the workspace's `brand/agents/<id>.md` (bundled defaults in `src-tauri/agents/`, compiled in). The guide is the human-tunable part of the prompt and goes first; the code appends the part it relies on (data fence, JSON shape, section list), which can't be edited away. Workspace → Agents edits a guide in place, shows "Customised" and the agent's last run (`ai_log.agent`, migration 0004).
- **Highlights**: the summary JSON gains `highlights` — the points the owner's notes flag, each tied to what the meeting said. Shown first on the Summary tab (tinted), available to the minutes template as `highlights`. The Notes card has **Re-run with my notes** (keeps edited sections).
- The pipeline after transcription: summary → naming check → the owner confirms → deterministic rename + people memory → email and minutes rebuilt (naming and email agents follow in later steps of this build).

---

## D-034 — Red means Record (and destructive), one record control; Settings is a fixed window
_2026-10-25_

**Decision** (user: "three kinds of record button; use one; record is critical and should follow the common rule of red"):
- One `.btnRecord` style — red fill, white dot and label — for the toolbar Record button and the dialog's Start recording; the sidebar entry was removed (toolbar + ⌘R are the way in). Red is reserved for record and destructive actions; orange stays the app's primary for everything else.
- The ⋯ popover follows macOS menu geometry and uses the system accent for the hovered row, like native menus — the one place the accent colour appears as a fill.
- The Settings window is 780 × 580 and not resizable, like System Settings: one layout, no responsive branches.
- **A terminated web content process reloads itself.** macOS kills WKWebView content processes under memory pressure (likely here: Whisper + Qwen + the webview on 16 GB); WebKit respawns an empty one and the window stays grey until something reloads it. `tauri::Builder::on_web_content_process_terminate` → `webview.reload()`, with a warning in the log. This was the "blank window" seen in dev and in the installed 0.3.0.
- Spacing moved up one step across the shared classes (panel 40, sections 48, cards 24, rows 16/24) after the user found the app too compact; pages must keep using the shared classes rather than their own numbers.

---

## D-033 — Majoris Design System on an Apple glass shell; the workspace brand is for documents only
_2026-10-25_

**Decision** (user: "apply the Majoris design system … keep Apple Liquid Glass … primary = orange 500, but don't change everything aggressively — a feeling that orange is the primary; the app UI should be separate from the workspace brand"):
- **Split of ownership.** Apple owns the shell and the control shapes: native glass window/sidebar/toolbar, capsule buttons, segmented controls, search and menus, hover and press behaviour, system accent for selection/focus. Majoris owns colour and type: DM Sans (bundled, variable), Slate neutrals, **orange as the one primary** (`--primary` = orange-500 fills, 600 hover, 700 pressed), green/yellow/red status. Majoris's own 4 px radii, blue shell and 300-weight body are *not* adopted — they would fight the Mac.
- **Orange only where navy was.** The old brand variables were renamed to `--primary*` and seeded with the Majoris orange ramp; nothing new is tinted. Secondary buttons became neutral (Apple grey), so orange appears on: the primary button, the active sidebar/settings item, orange chips, eyebrows, links, the progress bar, the playing transcript segment, the update banner.
- **Contrast.** Orange 500/600 fail 4.5:1 as text on white (2.8 / 3.2), so `--primary-text` is 700 (4.5) in light and 400 in dark; chips use 100/800 (6.0). Status text uses green-700, yellow-800, red-600 (the Majoris 600s for green and yellow fail as text). White on an orange-500 button is 2.8:1 — accepted as the user's explicit choice (Apple's own orange accent has the same ratio); the label is 14.5 px / 500.
- **Brand pack → documents only.** `applyBrand` is gone; `kernel/theme/brandLogo.ts` keeps just the logo for the workspace badge. `ContentRoot` and the Settings window no longer set any variable from a pack; the Brand page previews a paper-white first page (`DocumentPreview`) and edits logo, colours, slogan and fonts; its radius/button/status/canvas controls were removed (UI concepts). Known gap, now visible: the DOCX export takes its look from the meeting type's template, not from `tokens.json` — the pack's colours/fonts will be read when the app generates documents itself (PDF/PPTX).
- **AI is marked, nothing else is.** Every action Whisper or Qwen performs carries a sparkle (Material `auto_awesome`) and the app's one gradient (Majoris blue-600 → the orange primary): a fill on the primary AI button, a hairline ring elsewhere, animated while running; AI-written content says "Written by <model> · <date>" with the sparkle. Dropdowns are all the same capsule (`select.input`), the native element restyled rather than a custom component.
- **Playback is a waveform, not a system control.** The recording's peaks are computed once in Rust (`waveform.json` beside the recording, 1200 buckets, decoded through the same loader as transcription) and drawn on a canvas in the app's colours; the `<audio>` element stays as the hidden engine. Chosen over Web Audio decoding in the webview (a 46-minute file would mean hundreds of MB of floats in the page) and over computing during transcription (would save the few seconds of the first open; add when it bothers).
- **Never a blank window**: a root error boundary (`CrashScreen`) shows what broke with a Reload button, and uncaught errors are logged — a rendering bug must never leave the user with an empty glass window.
- Supersedes the D-004 typeface clause, its D-030 amendment, and D-030's "brand is live on the content area". CLAUDE.md "Typeface" and the design rules, the UI skill and the architect-reviewer agent were updated accordingly. Shared Majoris families beyond orange/slate/status (blue, cyan, charts) are added when charts arrive, not before.

---

## D-032 — Attendee corrections fix names everywhere, deterministically
_2026-10-25_

**Decision** (user: "after editing, fix the naming in the summary and the docs and update the workspace people"):
- Saving the Attendees sheet renames every spelling the user corrected (`heard_as` → new name) and every known spelling of the people involved across the whole summary document — section texts, topics, decisions, action owners, open issues, attendees, next focus — as whole words, any case (`people::rename_in_doc`, regex without look-around: boundaries checked by hand so Vietnamese letters count as word characters). The email and the minutes are rebuilt from that document, so they follow. The people memory is updated as before.
- Chosen over an automatic AI re-run: instant, deterministic, and it never rewords what the user liked. **Re-run with corrected names** (keeps edited sections) is offered right after a save for when the wording should change too.
- The Attendees editor is a modal sheet (860 pt table) rather than a form crammed into the 340 pt side column. The workspace badge shows the logo alone when the pack has one (name = tooltip and the switcher popover).

---

## D-031 — Storage that stays flat; OTA updates; a shippable app
_2026-10-25_

**Decision** (user: "I don't want this app booming my disk after a couple of weeks"):
- **Audio is the only thing that grows** (16 kHz WAV = 115 MB/hour; imports keep their size; the databases are hundreds of KB). So, once a transcript is saved, `kernel/storage.rs` converts the original to mono AAC at 48 kbps with `afconvert` (ships with macOS; ~20 MB/hour; 57 MB import → 17 MB in 10 s) as `recording.m4a` and deletes the original. Re-transcription decodes it with the same importer (symphonia, AAC). Originals are named `audio.<ext>`, compressed ones `recording.m4a`, so "not yet compressed" is a file-name check.
- **Retention**: app setting `retention_days` (default 90; 30 / 180 / 365 / 0 = forever). Two minutes after launch and daily, recordings of *summarised* meetings whose summary is older than that go to the Trash and `audio_path` is cleared; transcript, summary, minutes and `transcript.json` stay, and the Transcript tab says so. `ai_log` rows older than 90 days are pruned. Settings → Storage shows sizes per workspace and offers Compress now / Trash old recordings.
- **OTA updates** via `tauri-plugin-updater`: endpoint = `latest.json` of the releases of `mjx-nvt/new-dawn` — the user made the source repo public on 25 Oct (simpler than a separate releases repo and a token; everything in the repo, VIFC assets included, is public). Update bundles are minisign-signed with a key pair generated on 25 Oct 2026 (`~/.tauri/newdawn.key`; public key in `tauri.conf.json`); the app refuses anything else. This is the second exception to rule 3 after model downloads: one host, only for the check the user runs from Settings → About or the daily launch check they can switch off, and the download they confirm. Not notarised (no Developer ID): first install is right-click → Open; updates the app installs itself don't carry the quarantine flag.
- **Release pipeline**: `.github/workflows/release.yml` on a tag `v*` builds on an Apple Silicon runner and publishes `.dmg` + update bundle + `latest.json` as a release (GITHUB_TOKEN; the signing key as secrets). `scripts/release-local.sh` does the same from a local signed build. Version 0.2.0. *Update 2026-10-26:* releases are published from the Mac (`release-local.sh`) by default; every tag push had failed at the workflow's key check, so the workflow now only runs when started by hand on a tag (for when the Mac is off), one path per version. Hardened at the user's request: the signing key is a secret of the GitHub **`release` environment** only (not a repository secret), which needs the owner's approval for every run and accepts only `v*` tags; every action is pinned to an exact commit (checkout v5.1.0, pnpm/action-setup v6.1.0, setup-node v5.0.0, rust-toolchain stable, rust-cache v2.9.3, tauri-action v0.6.2 — all Node 24); checkout doesn't keep the token. This narrows, but doesn't remove, the risk of a GitHub compromise signing an update.

---

## D-030 — Brand packs applied and editable; Apple-style structure
_2026-10-24_

**Decision** (user feedback: fixed max width, too compact, too much text, no way to edit a brand):
- **Brand is live.** `kernel/brand/tokens.rs` reads `<workspace>/brand/tokens.json` (the bundled starter with the workspace's slug, else `default`, is copied in on first read: tokens + logos), validates, derives the 50–950 ramp when the pack has none, and resolves to a `BrandView`. `applyBrand` sets the brand variables inline on `.content-root` (main window, Settings, previews); in dark mode the pack's canvas/text/status stay with the system's dark tokens. Logos are served through the asset protocol (`brand/**` in scope); the sidebar badge shows the logo. **Settings → Brand** edits the pack in place (atomic write, unknown fields kept): logo light/dark, colours (system colour picker + hex), corners, button styles, slogan, document fonts; Reset to starter trashes the workspace's tokens and logos. Brand import from DOCX/PDF (the rest of P1.4) is still to come.
- **App settings ≠ workspace settings** (user feedback: one window mixing both was hard to read). The Settings window is app-only (General, Workspaces, Backup, Models, PDF converters, About). A workspace's own settings — Overview, Brand, Meeting types, People — are a page in the main window (`/workspace`, sidebar item, Workspace → Workspace Settings… ⌥⌘,), always for the current workspace; switching workspace switches the page.
- **Apple-style structure**: type scale raised (body 14.5, section titles 17/600 sentence case; uppercase only for the eyebrow); `.page` fills the panel; System-Settings-style Settings window (glass sidebar of panes grouped App / Workspace / AI, grouped inset rows); one primary action per state and a ⋯ menu on the meeting page, facts as a row, tabs as a segmented control, double-click to rename; Summary as two columns (sections + aside with email / attendees / notes) on wide panels; Meetings list without the duplicate search field; one hint per group.
- **Destructive = red**: `.btnDanger` / `DangerButton` (two-step, red fill to confirm) everywhere (Delete meeting, Move to Trash, Forget, Remove model, Delete type, Reset, Discard edits).

---

## D-029 — Navigation: icon rail, back/forward, container-based responsiveness
_2026-10-24_

**Decision** (user-approved plan):
- **Sidebar states**: labels (240) / icon rail (76) / hidden. Preference `auto | expanded | icons` in localStorage (a per-Mac convenience, guarded): auto = rail below a 1000 pt window; choosing what auto would pick returns to auto. ⌃⌘S and the ⟨⟨/⟩⟩ button toggle; ⌥⌘S still hides. Sidebar Record now toggles recording.
- **Back/forward** follow the webview's own history (TanStack `__TSR_index`), tracked once per window in a module store (survives the toolbar unmounting on /welcome); PUSH truncates forward entries, redirects are REPLACE. Toolbar ‹ ›, ⌘[ / ⌘] (ignored while a text field has focus, so an unsaved edit isn't left by accident). Finishing the welcome steps replaces history, so Back never re-enters it.
- **Toolbar search** is live: ⌘K, Enter → `/meetings?q=` (replace when already there), follows `?q`.
- **Responsive by container, not window**: `.content-root` = container `content`, `.main` = container `shell` (toolbar). Grids collapse, rows wrap, the Attendees editor turns into cards. Main window min 760 pt; Settings 860 × 560, resizable (min 600 × 420), tab strip scrolls.
- Drag regions are `"deep"` (Tauri 2.11: a bare attribute only drags on direct clicks on that element).
- Traffic lights y = 28 (tao's y is the title-bar strip height minus a button's; 28 centres them in 52 pt bars).

---

## D-028 — Meeting types own their templates; people memory
_2026-10-24_

**Decision** (user-approved; "the template I gave you is just the weekly meeting"):
- **One Word template per meeting type**: `<workspace>/brand/templates/meeting-minutes/<type id>.docx` (type ids are checked to be one plain file name). Minutes use the meeting's type template → the default type's → the older single `template.docx` (D-027) → the bundled starter. `document_template` stays unused; the type id is the link.
- **First use seeds "Weekly meeting"** (default, spoken language Auto) for a workspace without types: from its own `template.docx` (moved; a copy goes to the Trash) or the starter pack; serialised so two windows can't seed twice. A workspace keeps at least one type (deleting the last would only re-seed it). Deleting a type: its meetings keep everything but lose the type (FK `SET NULL`), its template goes to the Trash, and the default moves to the first remaining type.
- **Settings → Meeting types** (per workspace): name, language, Default, and per type Edit in Word / Replace… (checked copy) / Reveal / Reset to starter; Add (copy of the default type's template, or a picked .docx); field reference. **Settings → People**. Both are Settings tabs, not sections of Workspaces (the list got long). The meeting header has a type picker; New Recording / Import preselect the default type; the Minutes tab names the template it will use.
- **People memory** (`person`, migration 0003, per workspace — each workspace is one company and its own database): name, job title, organisation, other spellings. Matching folds case, accents, đ and spacing. Every summary run gets the list (≤ 80) with the owner's notes, and afterwards attendees get the remembered spelling/title/organisation, and action owners / decided by / who can answer the remembered spelling (Action items text refreshed). The Summary tab's **Attendees** editor saves to the minutes and email and, with *Remember* (default on), teaches it: an existing person is updated (an empty title or organisation never wipes a remembered one; an accent-less retyping keeps the remembered spelling), the model's spelling is kept as "also heard as", new names are added. Settings → People: edit, find, merge ("same person as…"), forget.
- Migrations: a migration file is frozen the moment any run may have applied it — 0003 was edited after a dev run and the checksum mismatch aborted the app at launch; it was restored byte-for-byte. Schema changes go into a new migration. And one workspace failing to open/migrate is logged and skipped, never fatal to the app.
- Memory safety: only added/changed attendee rows teach it; a person is matched by the saved name only (never renamed through the model's spelling); a spelling can belong to one person only (else: merge); names, titles and spellings are cleaned (no control characters or `<>`, capped) because they enter the prompt, where they are labelled reference only.
- Not done: alias replacement inside free text (Overview etc.) — the prompt list covers most of it; summary structure/email format per type (P1.4).

---

## D-027 — Pick the text model; manage the minutes template
_2026-10-24_

**Decision** (user-approved, "go + model picker"):
- **Text model is selectable.** `ollama::model()` reads a process-wide selection loaded at launch from the app setting `ollama_model` (default `qwen3.5:9b`). Settings → Models → Text model lists every model Ollama has (size, "Writes summaries"), with Use for summaries / Update (re-pull) / Remove (localhost `/api/delete`, waits for the AI queue so it can't pull a model from under a summary; the selected model can't be removed), Download another by name (validated `[A-Za-z0-9._:/-]`), and Free memory now. Selecting checks the model is installed and unloads the old one. `NUM_CTX` stays 32 768 for every model (Qwen 3.x supports it); a model that can't think fails Deep re-run with Ollama's own message. CLAUDE.md's "model `qwen3.5:9b`" is now the default, not the only option.
- **Minutes template per workspace**, still one file (`brand/templates/meeting-minutes/template.docx`, else the bundled starter by slug) — `document_template` stays unused until meeting types (P1.4) need several. Settings → Workspaces → Minutes templates: state (Starter / Custom · changed date), **Edit in Word** (copies the starter into the workspace first, so the bundle is never edited), **Replace…** (a picked .docx is checked first and only then copied in; the old custom one goes to the Trash), Reveal in Finder, **Reset to starter** (custom → Trash), and a field reference generated from the export model.
- **Template check** (`modules/meetings/template.rs`): lists the `{{fields}}` (split runs merged), flags unknown ones (they come out empty) and unused top-level fields, and fills the file once with sample data so an unclosed `{{#block}}` is caught before it replaces a working template. Shown for the current template too, so edits made in Word are re-checked.

---

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

**Amends D-004** (superseded by D-033 for the UI typeface, now DM Sans): Be Vietnam Pro remains the only UI typeface; a tenant's `document_fonts` (heading/body) apply only to exported documents.

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

**Decision**: The window chrome (background, sidebar, toolbar) is native Liquid Glass and follows macOS 27 Golden Gate conventions. A workspace's brand is applied only to the content area and only through four knobs: logo, colours, corner-radius style, button style. **Be Vietnam Pro is the single typeface for every tenant** and is not brandable. _(The brand-on-content-area part is superseded by D-033: the pack is for documents only.)_

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
