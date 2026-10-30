# Architectural Decisions

Newest first. Every entry: what was decided, why, and what it means for future work. Keep entries short; link to `docs/` for the long version.

## D-054 — Experts propose, the owner applies: Projects first
_2026-10-30_

**Context** (plan v3 §5): meetings now carry who said what, the owner's corrections and memory across meetings; what they don't yet do is keep anything up to date outside the meeting. The first expert keeps the owner's projects current.

**Decision — one shape for every expert**: a guide the owner can edit (Workspace → Agents, with its model and numbers), a typed view of the workspace it may read, and **proposals only** — it never writes to the owner's data; a card shows each proposed change with its evidence, the owner applies or dismisses, and both go to the ledger (D-049). It runs in the background lane after each summary (waits for a free Mac, D-050), and on request.

**Decision — projects** (migration 0016: `project`, `project_meeting`, `project_update`): the owner adds the projects they follow (name, status active / on hold / done, phase, next milestone and its date, risks, decisions, notes). After a summary, the **Projects agent** reads the open projects and the meeting's summary and answers which projects the meeting touched and what changed for them — a milestone (and date), a phase, a status, a new risk, a decision, a note — each with one sentence of the summary as evidence. The app keeps only what holds up: a project that exists, a field it knows, a real date, a status it knows, and evidence that is really in the summary (compared letters and digits only); the meeting is linked to every project it touched; the same change isn't proposed twice. The model writes a note per project before its answer instead of thinking at length (as D-052 found). **Apply** writes the change into the project in one transaction and keeps it as the project's history; **Dismiss** sets it aside. Deleting a meeting deletes the proposals that quote it (the applied values stay: they are the owner's decision); deleting a project deletes its proposals and links.

**Where the owner sees it**: **Projects** in the sidebar (with the number of updates waiting) → the list → a project's page: Updates waiting for you (each with its evidence, the meeting it came from and the AI marker), Where it stands, Risks, Decisions, Meetings, History — every section shown even when empty. A meeting's ⋯ menu has **Find project updates** (runs now). The Projects agent appears in Workspace → Agents with its guide, model and "applied of proposed" per model.

**Review fixes taken** (migration and architecture reviews): deleting a meeting keeps the decided updates (unlinked, without the quote — a trigger), and only the waiting ones go; the duplicate check works field by field (a list field once, whatever was decided, except an applied item the owner removed; a single-value field only against its current value, a waiting proposal or the same one dismissed from this meeting — a milestone without a date counts as the same milestone, a status as a status), and the prompt lists the decisions too; two open projects can't share a name (a finished one may), and a name needs letters or digits; Find project updates is the owner's own job (Foreground lane), the background run doesn't read the same summary twice, and a re-run summary drops the waiting proposals it no longer says; the owner's edits are one statement, so an Apply landing at the same time isn't overwritten; dates are stored padded; Delete project asks first; the page's controls wait while a save runs, the Edit sheet shows its error, and the project list (the sidebar's count) refreshes every minute; the Projects agent's number shows in Workspace → Agents.

**Means for future work**: CRM (0.9.1) and Email (0.9.2) take the same shape — their own tables, their own agent, proposals with evidence; the Cockpit (0.9.3) reads projects at risk across workspaces. Emails will propose project updates too (`project_update.meeting_id` is nullable for that).

---

## D-053 — Search by meaning: bge-m3 through Ollama, vectors in the workspace, compared in Rust
_2026-10-30_

**Context** (plan v3 §3.5): search finds meetings by their words (FTS5). A meeting about "khoan lõi bê tông" isn't found by "concrete core testing", and "partner onboarding" misses the call where nobody said those words.

**Decision — the model**: **bge-m3** (BAAI, MIT, 567M parameters, 1.2 GB) through Ollama's `/api/embed`, behind `ollama::embed` in the gateway. It is multilingual: an English query and a Vietnamese meeting meet in one space. On the dev meetings, the right piece ranked first for each test query (concrete core testing → the Vietnamese drilling discussion 0.48, against 0.29–0.40 for unrelated text). Downloaded through Ollama from Settings → Models → Search by meaning (Ollama fetches it; the app still only talks to localhost). Embedding models are kept out of every text-model list.

**Decision — storage and search without new dependencies**: the plan named `sqlite-vec`; it is a C extension, and this build may not add native code fetched from outside (see D-051), so vectors are plain f32 BLOBs in `embedding` (migration 0015) and a search compares the query with every vector of the workspace in Rust (cosine; about 150 pieces of 4 KB an hour of meetings — milliseconds for hundreds of meetings). An index (sqlite-vec, or an in-memory one) waits until a workspace is big enough to need it.

**Decision — what is indexed, and when**: a meeting's transcript in pieces of consecutive lines up to ~800 characters (each keeps where it starts, so a result opens the transcript there) and its summary by section. Each piece is hashed: when a transcript is run again or a section edited, only new pieces are embedded and gone ones deleted. A background loop (every 30 minutes, first after three) indexes changed meetings, each in its own catch-up turn — it waits for a free Mac (D-050) and lets the owner's work go first; Run now is in the AI activity list. A search embeds the query directly (one short text, no queue turn) and shows, under the text results on the Meetings page, **Related by meaning**: up to eight meetings the words didn't find whose closest piece scores 0.42 or more, with that piece as the snippet and the AI marker; the page's filters apply to them too.

**An exception to "one model at a time"**: a search embeds its query without a queue turn (one short text, like D-046's translations): it can run beside a summary being written. If Ollama is busy it times out after 20 s and says "busy", not "not running"; the text results stay. The indexer holds a catch-up turn, unloads the text model first and makes way for the owner's work between batches; `unload()` before Whisper also drops bge-m3. Each indexed meeting writes an `ai_log` row (agent "memory").

**Review fixes taken** (migration and architecture reviews): one check row per meeting (`embedding_check`) says when it was last indexed and with which model, so a meeting with nothing to embed counts as done and checking never rewrites its vectors; a composite index replaced two single ones; the check time is taken before the pieces are read; one meeting that fails is logged and skipped (Ollama gone or the model removed end the sweep), and Run now is let go however the sweep ends; a search reads the vectors a page at a time inside one snapshot and fetches only the winners' text; a line longer than a piece is cut; embedding models (any `bge-m3` tag or `*embed*`) stay out of the text-model lists, and bge-m3 can be removed from Settings; the Meetings page says when Ollama isn't running, when the model isn't downloaded, and how many meetings are indexed so far.

**Means for future work**: the summary's examples (D-049) can be chosen by similarity instead of recency; the experts (D-054) can find related meetings for a project or a person the same way. Pieces are packed greedily, so an edited transcript line re-embeds the pieces after it (seconds); a fixed time grid would avoid it if it matters.

---

## D-052 — "What I learned": the ledger proposes guide lines, the owner approves; a number per agent and model
_2026-10-30_

**Context** (plan v3 §3.3–3.4): since 0.8.1 the ledger keeps every decision the owner makes about the AI's work, and summaries already take the owner's rewritten sections as examples. Two things were missing: turning repeated edits into a lasting rule, and seeing whether an agent (and its model) is getting better.

**Decision — proposals, never automatic** (`modules/lessons.rs`, migration 0014 `lesson`): for the agents whose work the owner rewrites in their own words — **Summary** (sections rewritten, corrections, attendees changed) and **Tasks** (tasks deleted or reworded, action items sent to Tasks by hand) — a catch-up job reads the newest 25 decisions once five new ones have arrived (or any have waited a week; checked every 30 minutes, each run waits for a free Mac like any catch-up job, D-050). The text model notes how each decision changed the agent's work, then proposes at most three lines, each resting on at least three decisions, with its evidence ("You shortened the overview in 4 of 5 meetings"). The code keeps only lines that cite three real decisions and say something not already in the guide or proposed before. Workspace → Agents shows them under **What I learned** with **Approve** and **Dismiss** (kept only so it isn't proposed again); **Check now** reads at once. Approved lines live in their own file beside the guide (`brand/agents/<agent>.learned.md`) and every prompt gets them after the guide, under "Learned from your edits (these win over the rest of this guide)": Reset to default keeps them, the bundled default keeps reaching the workspace, and each has **Remove**. Naming and Email have no lesson: their ledger rows aren't the owner's own text.

**Decision — a number per agent and model** (`feedback::numbers`): Workspace → Agents shows, per model, how the owner judged the work since the ledger exists — summaries kept without rewriting a section, emails kept as first written, names the check doubted that the owner changed (the doubt was right) or kept, tasks the agent added that were kept, reworded or deleted. The model is the one that wrote the meeting's summary. Reworded tasks are now recorded with their id.

**Found while building**: with thinking on, the 9B model ran past 6,000 tokens on 25 decisions without answering; without thinking it was undecided (an empty answer one run in two at temperature 0.3), and the default guide's "Be detailed" kept it from proposing a shorter overview. Asking first for a note per decision, and saying that learned lines win over the guide, gave the right line in three runs of three.

**Review fixes taken**: deleting a ledger row (with its meeting) deletes a proposal resting on it and clears a decided lesson's evidence, which may quote the meeting (a trigger, as 0013 does for checks; migration review). From the architecture review: Check now lifts the wait of a scheduled reading of the same agent and waits for its result instead of failing; the decisions are read only once the job's turn has come, and a proposal is stored only while every decision it cites still exists; Approve waits while the guide has unsaved changes; a reading that fails on a too-long answer or prompt counts its decisions as read (no retry every 30 minutes); the job's hold and its Run now are let go however it ends; Approve is one transaction (a double click or two approvals can't lose a line); proposals are one line each; several saves of one section of one meeting are one decision (from the AI's text to the owner's last); the texts carry their word counts; each proposal shows how many edits it rests on, counted by the app; the numbers count a summary, its email or a task once it is three days old and only the edits made since the AI last wrote it; the Agents page queries less.

**Means for future work**: the speech side (vocabulary corrections undone, lines fixed by hand) has its counts in the ledger but no place on screen yet; thresholds that are guesses today (Live's Unclear, the names check's distance) get their counts once they have a setting to sit next to.

---

## D-051 — Who spoke: sherpa-onnx for speakers; a discussion keeps the microphone and the call apart
_2026-10-30_

**Context** (plan v3 §4): the summary should know who said what. Today a recording is one mix; Live already splits You / Them (D-043), batch transcription didn't, and two people talking at once lost one of them.

**Decision — the runtime, from the spike** (dev only, three meetings): **sherpa-onnx** (its C++ core, CPU) with **pyannote segmentation 3.0** (6 MB) and **NeMo TitaNet-large** prints (101 MB), clustered at threshold **0.8**, then every cluster with less than ~20 s of speech merged into the nearest large one by cosine. On m1 (30 min) it ran at 6.5× real time on 4 threads (≈ 9 min for an hour, a Background-lane job); thresholds 0.5 / 0.7 / 0.8 / 0.9 gave 159 / 111 / 88 / 65 clusters, and at 0.7–0.8 the six largest held nearly all the talk (≈ 400 / 290 / 240 / 230 / 100 / 95 s) — the rest are seconds-long fragments, hence the merge; 0.9 starts joining two real voices. WeSpeaker ResNet34 gave 13 clusters but put several people in each (657 s and 572 s for the two largest): not used. **FluidAudio** (CoreML) was not measured: building it from its source was blocked by this unattended run's permissions. It stays the candidate to beat; the model check (D-050) gets a speakers kind so it can be compared on the owner's meetings once it can be built.

**Decision — two channels kept** (0.8.3): whenever the call is tapped, the recorder writes `mic.wav` and `call.wav` (16 kHz mono, sample for sample on the mix's clock) beside `audio.wav`; repaired with it after a crash, compressed with it to `mic.m4a` / `call.m4a` after the transcript is saved, trashed with it by the retention setting and with the meeting's folder on delete. Recordings grow from ≈ 22 MB to ≈ 65 MB an hour (three AAC files at 48 kbps). Quick translations save nothing, as before.

**Decision — a discussion is transcribed per channel**: when a discussion has both files, each goes through the gateway on its own (its own checkpoint, `transcribing-mic.json` / `transcribing-call.json`, resumed at launch like the mix's; progress in halves), is cleaned like any transcript (vocabulary prompt echoes, spellings), then microphone lines that are the call heard through the speakers are dropped with Live's test (`live::overheard`: the same word pairs on the call within 900 ms), and the two are merged by time with You / Them on every line. The summary already names the owner's lines (D-043's `speaker_label`). Presentations, imports and meetings recorded before 0.8.3 use the mix as before.

**Review fixes taken** (architecture review): the Mac's sound is tapped whether or not a call runs, so a discussion whose call side carried under 20 s of speech is a meeting in a room — no line gets a speaker (else everyone's "I'll send it" would become the owner's action item); the channel files are best effort in the recorder (a write or finish error drops and deletes both, the mix records on, and the transcript uses the mix); a channel's file is looked up when its turn comes (a compression may have replaced the WAV); channel files are compressed in the background, so they never delay the transcript or the summary; retention trashes every channel file in the meeting's folder whatever state the mix is in; Compress now and the count of originals include them; a model check says when its references were transcribed apart (a trial hears the mix, so both models score a little lower on overlapping speech).

**Means for future work**: building sherpa-onnx into the app (the `sherpa-rs` crate, which links sherpa-onnx's prebuilt libraries) was blocked by the same unattended-run permissions, so who spoke when (diarize the call channel only, the mix for imports and old meetings; Them lines S1, S2…) and who it is (TitaNet prints matched to people) wait for the owner to allow it; the plan's next releases move up meanwhile. Echo removal is word-based: without headphones a quiet echo Whisper hears differently can survive as a You line (Live warns about headphones; batch doesn't yet).

---

## D-050 — Try a model on your own meetings before switching; catch-up work waits for a free Mac
_2026-10-30_

**Context** (plan v3, layer 1): models keep improving, and the owner should be able to switch when a better one fits, judged on their own meetings, not on a benchmark. This week's comparisons (D-045, D-047) took scripts and a session each.

**Decision — reference meetings** (migration 0013, `meeting.reference`): the owner marks meetings from their ⋯ menu ("Use as a reference"; a Reference chip shows it). Their corrected transcript and edited summary are the gold.

**Decision — the model check** (`modules/model_check.rs`, Settings → Models → Try a model on your meetings):
- **Speech**: a downloaded Whisper model against the Meetings model, on two 90-s windows (a third and two thirds in) of up to five references with a recording, decoded exactly as a meeting is (vocabulary prompt, language, cleanup) through the gateway's `whisper::trial` — one model at a time so each loads once, nothing saved. Figures: words as in the owner's transcript (difflib's ratio by words), speed, lines repeated. Because the transcript was written by the model in use and then corrected, the page says so.
- **Text**: an installed text model against the Summary agent's, on up to three summarised references, written exactly as a run would (`summarise::trial`: the same guide, the owner's examples, people, clean-up; the summary's notes and clean-up are now shared helpers). The model in use is represented by the summary it wrote before the owner's edits (`ai_sections_json`, D-049). Figures: attendees and action items as in the owner's summary, decisions, time.
- **Blind pairs**: each window (speech) and each meeting's Overview and Action items (text) as A and B; which is which comes from a hash of the check and the pair (stable, not on the page). The owner picks A, B or about the same; the tally decides, and when the candidate wins one button switches (the Meetings speech model, or the Summary agent's model).

**Decision — catch-up work waits for a free Mac** (the Background lane's first user): before it queues, a check waits while the battery is under 30 % on battery, the Mac runs hot (thermal state serious), memory is under pressure (`kern.memorystatus_vm_pressure_level`), or the owner used the Mac in the last 5 minutes — unless Settings → Models → Catch-up work says "Also while I work". The AI activity list shows it held with the reason, and **Run now** lifts the wait for that job (it still queues behind the other lanes).

**Decision — the text-model adapter**: each model's card is asked once (`/api/show`): a model without the "thinking" capability is never asked to think, and the context requested never exceeds the model's own. A model Ollama can't describe keeps the old behaviour.

**Review fixes taken** (migration and architecture reviews): no CHECK on a check's kind or status (the speaker models add a kind); deleting a meeting deletes the checks holding its words (a trigger; each stored meeting and pair carries the meeting's id; a meeting deleted during a check is dropped from its results; only the ten newest checks are kept); Cancel also lifts a held check's wait; a text check uses only references summarised since 0.8.1 (else the "model in use" would be the owner's edited text) and names the model that wrote each when it differs; the speech trial is polished like a meeting (prompt echoes dropped, the vocabulary's spellings, the owner among the people) and scored over the speech the models actually heard, all windows together, "—" when there's nothing to compare; a candidate that can't write a readable summary is a finding, while Ollama gone, a missing model or a cancel stop the check; Ollama answering with another model than the candidate is refused; catch-up work waits while a recording runs, re-reads the Mac every 30 s (Run now every 2 s), and a Whisper trial lets a higher lane go first between stretches; the capability cache is refreshed when a model is pulled or removed, and summaries budget from the model's real context; a check a quit interrupted is marked stopped at launch.

**Means for future work**: the speaker models (0.8.4–0.8.5) get the same check; a check could run unattended when a new model is downloaded. A text trial is one long model call: an interactive job still waits for it (up to minutes).

---

## D-049 — The feedback ledger: every decision about the AI's work is kept, and the summary learns from your edits
_2026-10-30_

**Context** (plan v3, layer 2): most feedback was applied once and lost — a section edit overwrote the AI's text, a name decision and a deleted task taught nothing, and no prompt ever saw how the owner wrote things. Models change (D-047's model per job; the model check in 0.8.2), so what the app learns must live in data, not in a model.

**Decision — the ledger** (`modules/feedback.rs`, migration 0012 `feedback`, additive): one row per decision, append-only, text only (never audio), deleted with its meeting. Written by the paths that already receive the decisions, best effort (a ledger that can't be written never fails the owner's action):

| Decision | Where | Counts for |
|---|---|---|
| A transcript line rewritten; a vocabulary correction undone | `review::update_segment`, `restore_segment` | transcript |
| A correction (with how much it changed) | `corrections::apply` | summary |
| A section rewritten (a tick isn't) | `summarise::update_section` | summary |
| Attendees renamed or moved in or out of the minutes | `review::update_attendees` | summary |
| An action item sent to Tasks | `tasks::add_from_item` | summary |
| A doubted name confirmed (with whether it was the check's suggestion) or kept as heard | `naming::confirm` | naming |
| A task the AI added, reworded or deleted | `tasks::update`, `tasks::delete` | tasks |
| The email written again | `summarise::rewrite_email` | email |

Guide edits aren't logged: the guide itself is the record.

**Decision — the AI's version kept**: `summary.ai_sections_json` holds the summary as the AI wrote it, before the owner's kept edits are put back, so an edit can always be compared with what the AI said.

**Decision — examples in the summary prompt**: the owner's own rewrites of sections in earlier summaries of the **same meeting type** — taken from the ledger, so the Corrections agent's rewrites never count as the owner's writing — go into the summary agent's instructions after its guide: the last version of each, the newest three, 600 characters each, fenced like any meeting text (`<example>`, covered by the "data, not instructions" rule): "match how the owner writes them (length, tone, what they include), whatever language they are in; never take facts, names or figures from them". Sections the model fills from its lists (topics, decisions, action items, open questions) and empty ones are left out. The input budget now follows the real prompt length (guide + examples + rules) for the parts and the merge. Saving a section unchanged is no longer an edit. They're the owner's own words, so they follow the model when it changes.

**Shown**: Workspace → Agents says, for the summary, names, email and tasks agents, how many of the owner's edits and decisions are recorded.

**Means for future work**: "What I learned" (0.8.6) proposes guide lines from the ledger, and the hit rates per agent and model are counted from it; examples move to retrieval by meaning with the memory search (0.8.7).

---

## D-048 — AI work in lanes, with checkpoints: what you wait for goes first, a quit loses nothing, heavy work goes easy while you work
_2026-10-30_

**Context** (plan v3, layer 3): one AI job at a time was a plain mutex held for a whole job: a Corrections click waited behind a 20-minute summary; a quit lost the transcription or summary in progress; nothing eased off while the owner used the Mac; nothing showed what the AI was doing.

**Decision — lanes** (`kernel/ai/queue.rs`): still one job at a time, but the AI queue orders waiters by lane, then arrival: **Interactive** (the owner waits: Corrections, Rewrite email, the slogan's translation) → **Foreground** (transcription and summary, choosing or removing a text model) → **Background** (catch-up work; its first user, and the gate on battery, heat and the owner's absence, come with the model check in 0.8.2). A long job holds its turn between checkpoints; at each one, if a higher lane waits, it **makes way** and comes back at the front of its lane. A dropped wait never keeps the turn (tested). Live keeps its exclusive hold (D-041).

**Decision — checkpoints instead of a job table**: the plan proposed a `job` table; derived state does the same with no migration.
- Transcription runs in batches of 8 speech stretches (about a minute of Whisper). After each, its place (model, language, prompt hash, the stretches the VAD found, lines so far) goes to `recordings/<id>/transcribing.json`; a waiting higher lane stops a batch after its current stretch. The file is deleted once the transcript is saved, or when the owner cancels; an error keeps it, so Transcribe again goes on from it.
- The summary saves each part to `recordings/<id>/summarising.json` (keyed by a hash of the prompt, transcript and notes) and makes way between parts, before the merge and between the agents that follow (names, email, tasks). The file is deleted once the summary is saved; the agents after it aren't resumed (best effort, as before).
- At launch, a meeting left transcribing or summarising with a saved place resumes from it (`meetings::resume_interrupted`), at most three tries per place, so a recording that crashes the app can't loop.

**Decision — going easy** (`kernel/ai/resources.rs`, Settings → Models → While you work, default **Gentle**): while the owner works (input in the last 2 minutes), on battery or when the Mac runs hot (thermal state serious), Whisper runs on half the threads on a thread of its own at utility priority (macOS moves it to the efficiency cores when the owner's apps need the performance ones), and the text model leaves memory a minute after a job instead of five. Ollama's `num_thread` is deliberately left alone: changing it makes Ollama reload the model. The GPU can't be throttled; Metal bursts stay.

**Decision — the AI activity list**: a toolbar capsule (✦ and what runs, "+N" waiting) appears while anything uses the AI; its popover lists the running job and the waiting ones in order, with the transcription or summary progress bar and Cancel. Polled every 2 s (`ai_activity`).

**Measured** (dev build, 88-min Vietnamese meeting, turbo): a slogan translation asked mid-transcription paused Whisper 1.6 s later, ran in 9.3 s (loading the 9B model included) and Whisper went on 4 s after: the request no longer waits ~9 minutes. The app killed at stretch 22 of 242 went on at stretch 22 after relaunch (about a minute to decode the audio again); killed after the summary's first part, it went on at part 2 of 4.

**Review fixes taken** (architecture review): a woken-but-dropped wait closes its receiver before checking it (a wake in between went to nobody and froze the queue); `make_way` re-queues and hands on in one locked step, so nothing of its own lane slips in and the turn comes straight back if the higher-lane waiter gave up; the transcription saves a stub place (the try counted) before it waits or decodes, so a waiting job resumes too and a recording that crashes decoding can't loop; a meeting's places live in `recordings/<meeting id>/` whatever its audio layout; a finished transcription doesn't make way; a failed checkpoint write only warns; the summary's place is keyed by its model too; a meeting left transcribing goes back to summarised, transcribed or recorded by what it holds; Cancel on a waiting job says "Stopping…" until its turn comes.

**Also — a summary answer of the wrong shape**: seen in the dev check, part 2 of the 88-min meeting came back twice with a list shaped wrongly and failed the summary. Now the answer keeps every field that reads and leaves out the one that doesn't (as if nothing was said for it), as long as its sections read; otherwise the model is asked again as before. The log records the answer's structure (field names and types, never its words) so the next slip can be read.

**Means for future work**: the model check (0.8.2), speaker matching (0.8.4–0.8.5) and embeddings (0.8.7) run in the Background lane; new long jobs save a place the same way.

---

## D-047 — A Whisper model per job; looped lines and channel sign-offs cleaned; screen checks without clicks
_2026-10-30_

**Context**: the blind large-v3-turbo vs large-v3 comparison on three real meetings: large-v3 won 17–7 on typical stretches at about twice the time; turbo looped on Vietnamese ("Như vậy, lập tức." 18 times in a row). The app used the first downloaded model in catalogue order for everything, so downloading turbo for Live also switched meeting transcripts to it. Plan v3 (`docs/plan.md`) starts here.

**Decision — a model per job** (`models::SpeechJob`, `models::speech_model`):
- Meetings (transcripts after Stop and imports) and Live each have a model, chosen in Settings → Models → *Used for* and saved in `registry.db` (`speech_model_meetings` / `speech_model_live`; empty = automatic).
- Automatic, among downloaded models: Meetings large-v3 → turbo → turbo-q5; Live turbo → turbo-q5 → large-v3. A chosen model that isn't downloaded falls back to automatic (the select says so).

**Decision — cleanup** (`whisper::clean_text`, `whisper::drop_loops`):
- A run of 3 or more identical lines (case and punctuation aside) keeps one. Two in a row stay: "Dạ." "Dạ." is real. Batch only: Live never rewrites a shown line.
- Channel sign-offs are dropped **sentence by sentence** (a sentence ends at `.` `!` `?` `…` before a space or the end, so "Amara.org" and "3.5" stay whole), because Whisper appends them to real speech as often as it writes them alone. New Vietnamese phrases, in their call-to-action form only: "hãy đăng ký kênh", "nhớ đăng ký kênh", "like, share và đăng ký", "ủng hộ kênh của mình", "cảm ơn các bạn đã theo dõi", "nhận thêm nhiều video mới", "không bỏ lỡ những video", "la la school". "Lượt đăng ký kênh tăng 15 %" is a real sales line and stays, as "subscribe" alone does in English.
- In Live, a line that only lost a sign-off isn't "runaway" (Unclear, untranslated): only a collapsed repeat counts.
- Replayed on the six test transcripts: 27 looped lines dropped (17 from turbo on the Vietnamese meeting, 5 from large-v3's "Thứ ba, lỡ khoan…" ×6, 5 from turbo on the mixed meeting); the two sign-offs cut, the real speech after one of them kept.

**Decision — screen checks in debug builds** (`windows::dev_open`): `NEWDAWN_DEV_OPEN=/route` (or `settings:<tab>`) opens a page at launch, `NEWDAWN_DEV_THEME=dark|light` sets the app's appearance (never the Mac's), and JavaScript written to `<dev data>/dev-eval.js` runs in the main window when the file changes (scroll, click, type — no Accessibility rights needed). The dev app runs as an accessory (no Dock icon, never activated), so a check doesn't take the keyboard. WebKit stops painting a window that other windows cover; the main window's WKWebView gets `_setWindowOcclusionDetectionEnabled: NO` (the SPI Playwright's WebKit uses; checked with `respondsToSelector:` first), through `objc2`, which was already in the build through Tauri. The separate Settings window stays grey while covered even so, so Settings panes are checked in the main window (`/settings?tab=…`). Release builds compile none of this.

**Means for future work**: the model check (0.8.2) compares models per job. Live's segmenting is unchanged.

---

## D-046 — Transcripts keep each voice's language; summaries are written in English; Apple translates on this Mac
_2026-10-29_

**Context** (the user, after a mixed English/Vietnamese meeting): "I don't need the translation in the transcript… keep the transcript original to the voice"; "all the transcript should be translated to English first to have the same language for the summary"; a summary tab for one more language (VI, FR, DE, JA, KO, ZH, ES), translated only on a click, with a default; "is there any good and free API / tool… instead of using AI model".

**Decision — the transcript** (`whisper::transcribe`):
- In an "auto" meeting each stretch of speech is Vietnamese or English, chosen from Whisper's own odds for the two (`vi_en_odds`). Before, whisper.cpp chose among 99 languages. large-v3 (the installed app's model) then wrote English it took for Vietnamese as a Vietnamese *translation*, and one line as Malay.
- A close call (neither language at 85 % of the two) is written both ways. The other language wins only when Whisper is clearly surer of it (mean log-probability +0.03), because a translation can score within a hair of the real words.
- Each line keeps its language (`transcript_segment.lang`), shown as the VI/EN tag the both-ways Live view already uses. Meetings set to one language are unchanged.
- **Measured** on the user's 61-min meeting with large-v3, every stretch written both ways:
  - 126 of 146 stretches are decided by the odds alone, and 20 are written both ways.
  - The margin matters both ways. At 16:40 a translation scored a hair surer than the English; at 20:13 and 43:39 the odds leaned Vietnamese for English speech, and the English version was clearly surer.
  - The original transcript had more than ten translated stretches and a Malay line.
  - Left: about 3 stretches where a speaker starts in Vietnamese and goes on in English. Whisper picks the language from the start of a stretch (up to 25 s), so the English part is translated. Shorter stretches would catch them at about twice the transcription time — the user's call.

**Decision — the summary** (`summarise::run`): written in English (`SUMMARY_LANGUAGE`), with its email and the minutes, from the lines **as spoken**.
- The user asked for the transcript to be translated into English first. That was built and measured with Qwen 9B, judged blind, and dropped:
  - The mostly-English meeting: 6 against 5 for translating first.
  - The 88-min Vietnamese meeting: 3 against 6, and 25 min against 18.
- Apple translating misheard Vietnamese line by line loses what Qwen recovers from context: "chùa Tứ Cụ" became "Tu Cu Pagoda", "bảo hành 60 tháng" became "a 60-month curing period", and the attendee list filled with noise ("iPhone", "Nuc").
- Revisit only with a better transcript, or as an opt-in for English-heavy meetings.

**Decision — one more language** (`translations::translate`, Summary tab):
- The tab reads English | the second language. On Translate, the summary (titles, highlights, every line; markdown, checkboxes and action-item owners kept, so "June" stays a name) is translated, kept per meeting (`summary_translation`), and marked out of date when the summary is written again or edited.
- Read-only: edits happen in English. The default language is in Settings → General → Summaries (Vietnamese until set). The language shown is in the URL (`?lang=`).

**Decision — the engine: Apple's Translation framework, a third AI capability** (`kernel/ai/translate.rs`):
- It is reached through a small bundled Swift helper (`helpers/translate.swift` → `newdawn-translate`, built by `build.rs`, Tauri `externalBin`).
- It runs on-device and is free. The app makes no network call; macOS keeps the models. It uses Apple Intelligence's model where there is one (macOS 26.4+); Apple documents that every `TranslationSession` translation is processed on the device. It holds no AI-queue turn.
- Apple's model can be unavailable for a while (once, about 15 minutes, during testing; offline models not downloaded); Translate then says so. A helper run gets 90 s plus 4 s a line, then it is stopped.
- `ai_log` rows use kind `complete`, model `apple-translation`; the table's CHECK allows only two kinds, and rebuilding it wasn't worth it.
- Measured on 20 real Vietnamese lines, judged blind against Qwen 9B: a tie (6 wins each, 8 ties; 3 serious errors for Apple, 4 for Qwen). About 1.2 s a line against about 2 s plus model loading, without occupying Ollama.
- Not chosen:
  - Google Cloud Translation: 500k characters a month free, then $20 per million; it needs an account and an API key and sends meeting text to Google, breaking rule 3.
  - Siri: no API, only a Shortcuts action.

**Means for future work**:
- The GitHub release workflow's `macos-15` runner has no macOS 26 SDK. The helper then builds, but answers "unavailable" (`#if compiler(>=6.2)`); build releases on a Mac with Xcode 26, or move the runner.
- Exporting the minutes or the email in the second language is the natural next step.

---

## D-045 — qwen3.5:9b stays the text model; the app guards corrections and mends JSON
_2026-10-29_

**Context**: a blind comparison of qwen3.5:4b against 9b, run on the app's own agent code with two real meetings (an English call of 30 min, a Vietnamese one of 88 min). Every later agent got the same inputs on both models. The report holds meeting content, so it lives outside the repo, in the dev data folder `eval/`. The ignored `eval_*` tests reproduce it.
- **9b won all 9 blind comparisons**: summary, email, corrections, live translation both ways, slogans.
- **4b's failures are the kind that matter**: invented facts and deadlines, sections marked "Not discussed" that were discussed, and reversed meaning in Vietnamese → English live lines. It is about 2× faster.
- **Decision**: 9b stays the default for every agent. Live keeps 4b as its fast default (it's close on English → Vietnamese), but Vietnamese meetings are better on 9b. The Live agent's model setting is the switch (7.7 GB against 4.1 GB beside Whisper).

**Decision — corrections guardrails** (`corrections::changes`). Both models turned corrections into changes nobody asked for:
- "Giang is the structural consultant" renamed five other people to Giang.
- Whole sentences were replaced with the correction's text.
- Some rules were written backwards.

A wrong → right change from the agent (a replacement or a person's other spelling) is now kept only when:
1. its wrong text is in this meeting. An other spelling must be in the transcript, since that is where names are misheard;
2. it has at most six words, unless the correction quotes it. Bigger changes come as rewritten sections;
3. it doesn't turn someone known into someone else unless the correction names them. Known means an attendee, the people memory, or the owner. Naming by given name counts ("Phong"); a family name alone doesn't;
4. a change of accents or case goes to the spelling the correction wrote;
5. a spelling has one right form, and a replacement wins over a person's other spelling.

People the correction doesn't name are no longer saved to the people memory. 4b had added five, with their titles and organisations rewritten.

On the comparison's stored answers this blocks every damaging change and keeps the intended ones. A backwards rule where the correction names both spellings ("Soviko is Sovico") can still pass. Only the meaning tells which spelling is right.

**Decision — JSON**:
- `summarise::mend_brackets` repairs the bracket slips seen before the summary asks the model again: a list left open before a key or the object's end, `}]` for `]}`, `]` for `}]`, a trailing comma, closing brackets left off at the end.
  - It reads from the first `{` to the end of the answer and stops where that object closes, so a missing final `}` no longer loses what came after the last `}`.
  - An answer that stops inside a string isn't guessed at. One cut by the length limit already fails (`output_truncated`).
- It reads all three of 9b's failed parts of the 88-min meeting. That saves a retry each, about 2 min apiece. It also reads 4b's open-list answers.
- 4b's other failure (`{"title": "next_focus": …`, a confused shape) is not mended.
- The names check reads its suspects with or without the `{"suspects": …}` wrapper. 4b answers the bare list, so its checks were all thrown away.

---

## D-044 — Debug builds keep their own data, so developing never touches the installed app
_2026-10-29_

**Decision** (user: "separate the coding testing app with the installed I have in my laptop… I need it to work during the coding, without interrupt"):
- A debug build (`pnpm tauri dev`, `cargo test`, the ignored real-model tests) reads and writes `~/Library/Application Support/co.majorisdigital.newdawn.dev` and logs to `~/Library/Logs/co.majorisdigital.newdawn.dev` (`kernel::storage::{data_dir, log_dir, LOG_FOLDER}`); a release build is unchanged. The bundle id stays `co.majorisdigital.newdawn`. WebKit storage and caches were already separate (the dev binary is `new-dawn`). A debug build doesn't save the window position (the plugin's file lives in the release app's folder).
- Why: the dev app shared the installed app's folder, so an in-app check ran an unreleased migration on the user's workspace and the installed 0.7.3 refused to open it (0.7.5). The two can now run side by side (no single-instance lock, no global shortcuts); only Ollama is shared.
- Publishing no longer installs anything on the Mac; the user updates the installed app themselves.

---

## D-043 — Live 0.7.5: you and the call apart, how clearly each line was heard, both ways, a quick translation that keeps nothing; task priority
_2026-10-26 · release 0.7.5_

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
