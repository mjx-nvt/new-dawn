# New Dawn — Plan v3 (phase 2: speakers, an AI that learns from you, work that stays in the background, experts)

_2026-10-30. Supersedes `plan-v2.md` (kept as history; its v1 scope shipped as 0.1–0.7.7). Everything carried over stands: Tauri 2 + Rust engine + React cockpit; one SQLite file per workspace; Liquid Glass shell; DM Sans; Majoris palette; **100 % local AI**; bundle id `co.majorisdigital.newdawn`; MacBook Pro M1 Pro 16 GB. Reference docs: `ai-engine.md`, `document-export.md`, `meetily-teardown.md`. Plain language. Build prompts are written per release at build time, as the 0.7.x series was._

## 1. Where 0.7.7 stands (the review)

**Works, daily.** Record (mic + call, no bot) → transcript (Whisper, each stretch in its own language) → executive summary, email and branded minutes (Qwen 9B) → tasks. Live captions with translation. Corrections that teach the vocabulary and the people memory. Agents with editable guides and a model each. Apple's on-device translator for the second language. Every AI call logged (`ai_log`). OTA updates. Three blind model comparisons this week (4B vs 9B, turbo vs large-v3, Parakeet vs large-v3), run by hand on the app's own code.

**Measured on this Mac** (used throughout this plan):

| Job | Model | Time per hour of meeting | Memory while loaded |
|---|---|---|---|
| Transcribe | Whisper large-v3 | ≈ 11.5 min | ≈ 3–4 GB |
| Transcribe | Whisper large-v3-turbo | ≈ 6 min (loses to large-v3 17–7 blind on typical stretches; loops on Vietnamese) | ≈ 2 GB |
| Summary + names + email + tasks | Qwen 3.5 9B | ≈ 12 min | ≈ 8–9 GB |
| Who spoke when | sherpa-onnx on the CPU (tested) | ≈ 9 min | small |
| Who spoke when | FluidAudio on the Neural Engine (published: 323× real time on an M5 Pro) | likely under 1 min — to measure | small |
| Translate the summary | Apple Translation | ≈ 1.2 s a line | none of ours |

Models on disk today: 10.7 GB (large-v3 3.1 GB, Qwen 9B 7.6 GB). The app itself is 13 MB. Speakers add ≈ 100 MB.

**What the three layers you asked for need, and is missing today:**

| | Today | Gap |
|---|---|---|
| **Learning that survives a model change** | People, vocabulary and guides are data, re-applied on every run — they already survive. A model is a setting per agent. | No way to *check* a new model on your own meetings before switching (this week it took scripts); some model quirks are hard-coded (thinking, context, JSON repairs); one speech model for both batch and Live; voice prints (new) will be tied to their model. |
| **A harness between you and the model** | Corrections → vocabulary + people. Section edits keep an `edited` flag. Names: Use this / Keep as heard. Tasks: Add to Tasks. | Most feedback is applied once and then lost: a section edit overwrites the AI's text, a name decision is not kept, a task you delete or reword teaches nothing, a Live line you'd have corrected has nowhere to go. The model never sees how you wrote the same section last time. No number says whether an agent is getting better. |
| **Work in the background without disrupting you** | One AI job at a time (`AiQueue`, a mutex); jobs are in memory (`TranscriptionJobs`, `SummaryJobs`), lost on quit; Whisper on 8 threads + Metal; Ollama at its defaults. | No priority (a Corrections click waits behind a 20-minute summary); no notion of battery, heat, free memory or whether you're using the Mac; a summary chunk is one long call; nothing resumes after a quit; nowhere to see what's queued and why it waits. |
| **Speakers** | Live: You / Them from the two channels. Batch: nothing — the recorder saves one 16 kHz mix. | Diarization, voice prints, identification, and the router around Whisper (orchestration). The benchmark this week chose the pieces (§4). |
| **Experts** (projects, CRM, email) | Meetings, people, tasks exist per workspace. | No projects, organisations, interactions or email; no retrieval across meetings (no embeddings yet). |

## 2. The idea in one paragraph

**What you teach the app lives in data, never in a model's weights.** People, vocabulary, your edits, your decisions, your voice prints — all of it is stored as plain data and re-applied at run time, so a new model inherits it the day it arrives, and a built-in check tells you on your own meetings whether the new model is better before you switch. **Every AI job is a row in a job list** with a lane (you're waiting / this meeting / background), a checkpoint and a resource policy, so heavy work runs behind your real work, resumes after a quit, and yields when you need the AI now. **Speakers are the next big input** (who said what makes summaries, tasks and later projects and CRM more accurate). **Experts propose, you apply** — the names-review pattern, everywhere.

## 3. The three layers

### 3.1 Layer 1 — Models come and go; what you taught stays

1. **No fine-tuning.** On a 16 GB Mac a LoRA run on the 9B takes hours and would need hundreds of edited summaries to beat examples + retrieval; and weights can't be moved to the next model. Revisit only with > 200 edited summaries *and* a bigger Mac. Alignment is by data: the four stores below.
2. **The learning stores** (all plain text/numbers in the workspace DB, all already re-applied or planned to be): `person` (names, titles, organisations, spellings, later voice prints), `vocabulary` (term ← heard as), agent guides (`brand/agents/*.md`), the **feedback ledger** (§3.2), and **examples** (your edited sections). Changing a model = the per-agent setting that already exists.
3. **Reference meetings + the model check** (Settings → Models → *Try on my meetings*). You mark 3–5 meetings as references; their corrected transcript and edited summary are the gold. The check runs the real pipeline with the candidate model on them in the background lane (never touching the saved results) and reports numbers, not opinions: speech — word agreement with your transcript per 90-s window, repeated lines, filler lines, time; text — attendees found vs gold, action items matched (owner + wording), decisions count, section lengths, JSON retries, time. Then a **blind side-by-side** of three stretches (A/B flipped by a hash, as `analyse.py` did this week) — you pick, the app tallies. You switch per agent, or not. This is `modules/meetings/eval.rs` and the ignored `eval_*` tests made into a feature.
4. **Model adapters, not code**: a table in `kernel/ai/` with per-model-family settings — thinking on/off, context size, temperature, keep-alive, which answer repairs to apply (bracket mending, bare lists), whether the model answers JSON reliably. A new model is a row plus a check run.
5. **One speech model per job**: Meetings (batch, accuracy — large-v3) and Live (speed — turbo) chosen separately in Settings → Models, instead of "the first downloaded model wins" (`preferred_speech_model`).
6. **Voice prints survive their model**: a print is numbers from one embedding model, so the app keeps each person's 3–5 best clips (2–4 s each, ≈ 100 KB a person) and recomputes prints in the background when the model changes. Whisper and Qwen need nothing: text is text.

### 3.2 Layer 2 — The harness: your feedback makes the next run better

1. **The feedback ledger** (`feedback` table, append-only, text only, never audio): one row for every human decision the app already receives, written by the code paths that exist — transcript line fixed (`correct_segment`), correction applied (`corrections::apply`), section edited (`update_section`; the AI's text is kept *beside* yours from now on), name confirmed or kept as heard (`naming::confirm`), attendee moved (`people::remember`), task added / deleted / reworded / ticked (`tasks::*`), email rewritten, guide edited. Fields: meeting, agent, kind, before, after, context (section, model), when.
2. **What the model sees — explicit and inspectable.** The Vocabulary tab already shows "exactly what Whisper is told"; each agent gets the same: guide + people + vocabulary + **examples**. Examples = your edited version of the same section from the last 2–3 meetings of the same meeting type ("this is how the owner wrote *Decisions* last time"). Retrieval starts simple (same type, most recent, edited); embeddings (§6, Memory search) refine it later.
3. **"What I learned" — proposed, never automatic.** After every 5 new ledger rows (or weekly), a background job has the quality model read the ledger and propose at most 3 guide lines with their evidence ("you shortened the overview in 4 of 5 meetings → *Keep the overview to one paragraph*"). Workspace → Agents shows them as a card; Approve appends to the guide (the editable prompt), Dismiss forgets. Thresholds that are guesses today — Live's *Unclear* / *May be inaccurate*, the names check's suspicion distance, the speaker-match threshold — show their accept/reject counts from the ledger next to the setting, so calibrating is a decision you can see, not magic.
4. **A number per agent.** Workspace → Agents shows, per agent and model: runs, average time, and the hit rate from the ledger — names proposals accepted / rejected, auto-added tasks kept / deleted, corrections undone, speaker names accepted. You see the AI getting better, or a model making it worse.

### 3.3 Layer 3 — Work that stays in the background

1. **A job table** (`job`, per workspace): kind (transcribe · summarise · names · email · tasks · diarize · identify · translate · embed · check · backfill), meeting, lane, state (queued · running · paused · waiting-for · done · failed), **step** (the checkpoint), progress, model, attempts, error. The scheduler (`kernel/ai/jobs.rs`, started in `lib.rs`) reads the table; the existing `start()` functions insert rows and run from a step. A quit loses nothing: transcription resumes at its last saved batch of stretches, a summary at its last finished chunk.
2. **Three lanes.** *Interactive* (you're waiting: Corrections, Rewrite, Translate, Live) → *foreground* (the meeting you just recorded or pressed Write summary on) → *background* (backfills: diarize old meetings, re-transcribe with a new model, embeddings, model checks, "What I learned"). A background job yields at its next checkpoint: a Whisper stretch (≤ 25 s of audio, seconds of compute) or a summary chunk (minutes on the 9B — the interactive job waits for the chunk, or you press *Now* and the chunk is redone later). Live keeps its exclusive hold (D-041).
3. **The resource policy** (Settings → AI on this Mac → *While I work*). The background lane runs only when: on power or battery above 30 %; the Mac is not hot (`NSProcessInfo.thermalState` via `objc2-foundation`, already in the tree); free memory fits the model (`sysinfo`); and either you've been idle for 5 minutes (IOKit `HIDIdleTime`) or *Also while I work (slower)* is on — then Whisper gets 4 threads instead of 8, Ollama `num_thread` 4, and the worker threads run at Utility QoS (`pthread_set_qos_class_self_np`), which puts them on the efficiency cores behind your apps. *Quiet mode* applies the same to the foreground lane. The GPU can't be throttled: Whisper's Metal bursts stay, and they're short. Rule unchanged: one heavy model at a time; the small models (VAD, speakers, embeddings, ≈ 100 MB) run beside either.
4. **Smaller steps**: the summary's chunk → draft → merge → names → email → tasks stages become checkpointed steps (the pieces exist: `chunk_lines`, partials, the three agents); transcription saves every N stretches instead of at the end (Live already appends per line).
5. **Where you see it**: a small *AI activity* popover from the toolbar (like Safari's downloads) listing jobs with progress, Pause / Resume / Now / Cancel, and the reason a job waits ("on battery", "you're working", "waiting for the summary of …"). The per-meeting progress cards stay.

## 4. Speakers

**What this week's benchmark decided** (three real meetings, blind judging):
- **Whisper stays the ear.** large-v3 for meetings; turbo for Live. Parakeet is 2–3× faster but lost 22–7 on Vietnamese and mixed speech, has no Vietnamese + English model, and takes no vocabulary prompt. Not adopted; the runtime work isn't wasted — it's the same shape as the speakers helper.
- **Diarization and voice prints are separate small models**, not Whisper: pyannote segmentation (6 MB) + a speaker-embedding model (26–100 MB) + clustering. sherpa-onnx on the CPU works but is slow (≈ 9 min an hour) and its WeSpeaker clusters mixed people; TitaNet prints told people apart across meetings (0.7–0.8 same, ≤ 0.21 different). **FluidAudio** (Swift, Apache-2.0, CoreML on the Neural Engine; pyannote community-1 offline: DER 10.6 % on AMI, 323× real time on an M5 Pro; CAM++ prints: 0.81 same vs 0.26 different, EER 0.48 %) is the likely runtime. It fits the pattern we have: a bundled Swift helper like `newdawn-translate`, models downloaded by `models.rs` from Hugging Face at pinned commits (rule 3) and loaded offline by the helper.
- **Decided by a one-evening spike, not by the README**: both runtimes on the same three meetings, five minutes of each labelled by hand; count speaker-turn errors and check Ryan / Mr Tom match themselves across meetings. The result is D-051.

**The pipeline** (after transcription; its own job in the background lane; whole meeting):
1. **Two channels kept** from the recorder: microphone = You, call = Them (today they're mixed into one WAV; recording size ≈ 20 → 40 MB an hour after compression). Diarize the call channel only; imports and old meetings use the mix.
2. **Who spoke when** → `speaker_turn` rows; each transcript line gets the speaker with the largest overlap (`transcript_segment.speaker_label`: S1, S2 …; the mic channel's lines are You).
3. **Who it is** → each cluster's print is compared with the workspace's enrolled prints (cosine; start at 0.6, calibrated from the ledger); matches above the threshold are *proposed*. A **Speakers** card on the Summary tab (the names-review shape): "S2 — Ryan (0.78)?" with Use / Someone else / Leave; naming a speaker enrols or refreshes that person's print and keeps the clips. Unnamed speakers stay S1, S2 in the transcript.
4. **The summary hears names**: transcript lines are given as "Ryan: …", so decisions and action items get the right owner — measured by the ledger (fewer edits on Action items).
5. **Privacy**: prints and clips live in the workspace DB and folder, deleted with the person; nothing leaves the Mac.
6. **Live**: You / Them stays. Splitting Them live (LS-EEND streaming, DER 20.7 % on AMI) comes after the offline path proves itself.

**STT orchestration = the router around Whisper**, not several engines: per channel (in a discussion, batch transcribes microphone and call separately, as Live does — no more lost overlapping speech), per language (0.7.7), per job (the model per job, §3.1.5), a second pass on unclear stretches (beam 5, both languages), and the cleanup fixes (cross-line loops, Vietnamese sign-off phrases). All inside `kernel/ai/whisper.rs` + `modules/meetings/transcribe.rs`.

## 5. Experts (the auto-working features)

**One shape for every expert**: a guide (editable, Workspace → Agents) · a typed **view** of the workspace it may read (projects, people, meetings, tasks, emails) · **proposals** — it never writes directly; it proposes changes, a review card shows them with their evidence (the line in the transcript, the email), you apply · examples + ledger + the model check from Layers 1–2 · runs in the background lane after each summary (emails: on request, later on a watch). The quality model, thinking on.

| Expert | Data it owns | What it proposes after a meeting (or an email) | Page |
|---|---|---|---|
| **Projects** (the cockpit v1) | `project` (name, status, phase, next milestone, risks, decisions), `project_meeting`, `project_update` | "This meeting touched *WiredScore certification*: milestone moved to 15 Nov; new risk: contractor liability" | Projects list → project dossier (ghost sections) |
| **CRM** | `organisation`; `person` grows (organisation, role, last contact, notes); `interaction`, `commitment` (made / received, due, kept) | "Mr Phuc committed to the ultrasound report by Friday; last contact with Keppel Land 3 weeks ago" | People becomes the CRM: a dossier per person and organisation (meetings, tasks, commitments) |
| **Email** | `email_item` (what was read, from which thread, what came of it) | Tasks, project updates, CRM facts from a thread you picked | Inbox-free: you pick a mailbox or thread in Apple Mail (Apple Events, local — the app already drives Mail) or drop an .eml; later an opt-in "watch this mailbox" in the background lane. No IMAP, no network. |
| **Cockpit** | nothing new — the hub fans out across workspaces (rule 1) | — | Today across all companies: tasks due, projects at risk, meetings this week, commitments due |

**Memory search comes first**: experts need "what did we say about X in the last meetings". Embeddings (`bge-m3` through Ollama, as `ai-engine.md` §3c planned) + `sqlite-vec` per workspace, computed in the background lane; also used by the examples in §3.2.

## 6. Data model additions (append-only migrations; each reviewed by `migration-safety`)

| Migration | Tables / columns |
|---|---|
| 0012 | `job` (§3.3) |
| 0013 | `feedback` (§3.2); `summary.ai_sections_json` (the AI's text kept beside your edits) |
| 0014 | `meeting.reference`; `model_check` (candidate, metrics JSON, blind picks) |
| 0015 | `speaker_turn`; `transcript_segment.speaker_label`; `person.voice_print_json`, `person.voice_model`; `voice_clip` |
| 0016 | `embedding` (`sqlite-vec` virtual table) |
| 0017 | `project`, `project_meeting`, `project_update` |
| 0018 | `organisation`, `person.organisation_id`, `interaction`, `commitment` |
| 0019 | `email_item` |
| registry | settings for the resource policy and the speech model per job |

Folder additions: `models/speakers/` (the diarization and embedding models), `workspaces/<slug>/voices/<person-id>/*.wav` (clips), a second WAV per discussion recording until compression.

## 7. Releases, in order

Numbering: this phase starts at **0.8.0** (a new phase), then patch bumps as usual; the first expert is **0.9.0**. One release ≈ one to three sittings, as in the 0.7.x series; each is published before the next and never installed here (D-044).

| Release | Outcome | Done when | Sittings |
|---|---|---|---|
| **0.7.8** Cleanup | Cross-line loops dropped (the "Như vậy, lập tức" ×18), Vietnamese sign-off phrases, a speech model per job (Meetings / Live) | The m2 transcript has no repeated runs; Live uses turbo while Meetings use large-v3 | 1 |
| **0.8.0** Jobs | `job` table, three lanes, checkpoints for transcription and summary chunks, the resource policy, the AI activity popover | Quit mid-summary → relaunch → it resumes at the chunk; Corrections pressed during a background job runs at its next checkpoint; on battery below 30 % the background lane says so | 2–3 |
| **0.8.1** The ledger | `feedback` written by every existing path; the AI's section text kept beside yours; examples in the Summary and Email prompts; "What the agent is told" per agent | Every edit, decision and tick appears in the ledger; a second meeting of the same type shows your previous Decisions wording in the prompt view | 2 |
| **0.8.2** Model check | Reference meetings; *Try on my meetings* for a speech or text model in the background lane; numbers + blind side-by-side; model adapters table | Turbo vs large-v3 reproduced inside the app in one click, without scripts | 2 |
| **0.8.3** Speakers spike + channels | FluidAudio vs sherpa on the three meetings (dev-only; D-051); the recorder keeps microphone and call apart; batch transcribes a discussion per channel | Two WAVs per discussion; overlapping speech no longer lost; the runtime chosen with numbers | 1–2 |
| **(waits)** Who spoke when | `newdawn-speakers` helper; diarization job; S1/S2 labels on transcript lines; backfill for old meetings in the background lane; the summary hears speaker labels | A 60-minute meeting diarized in about a minute; the transcript reads with speakers; old meetings fill in overnight | 2 |
| **(waits)** Who it is | Voice prints, enrolment from the Speakers card, proposals with confidence, re-enrolment when the model changes, delete-with-person | The second meeting with Ryan proposes "Ryan" by itself; deleting a person removes prints and clips | 2 |
| **0.8.4** "What I learned" + agent numbers | Proposed guide lines from the ledger with evidence; hit rates per agent and model; calibrated thresholds shown with their counts | A proposed rule appears after five edits; Approve changes the guide; Agents shows accept/reject per agent | 1–2 |
| **0.8.5** Memory search | `bge-m3` through Ollama, vectors in the workspace compared in Rust (D-053; `sqlite-vec` when a workspace needs an index); embeddings in the background lane; search across meetings by meaning (examples by similarity: later) | "partner onboarding" finds the meeting that never used those words | 1–2 |
| **0.9.0** Projects | Project table and dossier; the Project expert proposes updates after each summary | A meeting's summary proposes a milestone change; Apply updates the dossier with its evidence | 3 |
| **0.9.1** CRM | Organisations, interactions, commitments; the CRM expert; People → dossiers | A commitment made in a meeting shows on the person with its due date | 2–3 |
| **0.9.2** Email | Pick a thread in Apple Mail or drop an .eml; the Email expert proposes tasks / project / CRM changes | A thread yields a task and a project update after review | 2–3 |
| **0.9.3** Cockpit | Across workspaces: due tasks, projects at risk, commitments, this week's meetings | One page answers "what needs me today" across companies | 1–2 |

**Order changed 2026-10-30 (D-051):** building the speakers runtime (sherpa-onnx, or FluidAudio) needs code fetched from outside, which the unattended build may not do; the two speaker releases wait for the owner to allow it and take the next free numbers then. "What I learned" and memory search moved up to 0.8.4 and 0.8.5.

Roughly **twenty-five to thirty sittings** — two to three months of evenings and weekends, at the 0.7.x pace.

## 8. Decisions this plan proposes (recorded in `DECISIONS.md` when each is built)

Numbered in build order (the release that records each in brackets).

| # | Decision |
|---|---|
| D-047 | A speech model per job; looped lines and channel sign-offs cleaned (0.7.8) |
| D-048 | Jobs as rows: three lanes, checkpoints, the resource policy (power, heat, memory, idle, QoS), the AI activity popover (0.8.0) |
| D-049 | The feedback ledger; the AI's text kept beside the owner's edits; examples in prompts (0.8.1) |
| D-050 | Learning lives in data, never in weights: no fine-tuning; reference meetings + the in-app model check; model adapters as a table (0.8.2) |
| D-051 | The speakers runtime (after the spike: sherpa-onnx); two channels kept (0.8.3); S-labels; prints and clips per person, deleted with the person (when the runtime can be built) |
| D-052 | "What I learned" proposes guide lines from the ledger, the owner approves; a number per agent and model (0.8.4) |
| D-053 | Memory search: `bge-m3` per workspace, computed in the background lane (0.8.5) |
| D-054 | Experts propose, the owner applies; one shape (guide · view · proposals · ledger · check); Apple Mail via Apple Events, no IMAP (0.9.x) |

## 9. Deliberately not in this plan

- **Cloud AI, API keys, network beyond localhost** — unchanged (D-003, rule 3).
- **Fine-tuning / LoRA** — §3.1.1.
- **Parakeet or a second ASR engine** — measured worse for Vietnamese and mixed speech; the router is around Whisper.
- **Bundled Ollama** — still "when the app should feel self-contained" (`ai-engine.md` §3b); nothing here depends on it.
- **A bigger text model** — out of reach on 16 GB (`ai-engine.md` §10); the model check will show the day a better one that fits appears.
- **Automatic actions** — no expert writes to a project, a person or Mail without your Apply.

## 10. Open with you

1. **Numbering**: 0.8.0 for the first release of this phase, or keep patch bumps (0.7.9 …)?
2. **Two channels per discussion** doubles the compressed recording (≈ 40 MB an hour). Worth it for speakers and overlap — say if not.
3. **Which mail app**: the Email expert reads Apple Mail locally through Apple Events; Outlook or Gmail in a browser are only reachable by dropping an .eml. If you don't use Mail.app, 0.9.2 changes shape.
4. **Background default**: run heavy background work only when you're idle (the safe default), or also while you work, slower?
5. **Housekeeping from this week's tests**: `eval/pk` in the dev data folder is 5.4 GB and Homebrew's `python@3.13` was installed for it — both can go (`brew uninstall python@3.13`). The VIFC restore (people + vocabulary) still waits for your new workspace.
