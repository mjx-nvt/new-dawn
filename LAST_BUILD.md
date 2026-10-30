# Last Build — Give New Dawn an email to read · release 0.9.2
_2026-10-31_

## What changed
- **Email** (sidebar, D-056). Give New Dawn one email and it proposes what the email means for you:
  - **your tasks**: they wait under the email. **Add to Tasks** puts one in Tasks, with the email as its reason and the date read as a day when it is one; **Dismiss** sets it aside;
  - **project updates**: they wait on the project's page, with the same checks as a meeting's;
  - **others' commitments**: they wait in People to be confirmed.

  Every proposal quotes a sentence the email really contains.
- Two ways in:
  - **Read the email selected in Mail**. The first time, macOS asks you to allow New Dawn to control Mail.
  - **Open an .eml file…**

  Nothing is fetched and nothing is sent: New Dawn reads only the email you give it, on this Mac.
- **Emails read** lists each email with what came of it: its tasks, and links to its updates and commitments. **Forget** removes an email together with the proposals still waiting; what you already decided stays. The same email isn't read twice.
- On People and on a project's page, what came from an email says "from the email “…”". In Tasks, a task you added from an email names the email.
- A new **Email reading** agent in Workspace → Agents, with its guide and model.
- Checks: 159 Rust tests pass. New ones cover:
  - the .eml reader: a plain mail, the plain part preferred over HTML, and hostile mail (deep nesting, huge headers) read safely;
  - only entries that quote the email are kept, and an answer that can't be read is an error, not "nothing to do";
  - a proposed task is added once, and forgetting the email keeps it in Tasks.

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app, with a test email about "Block B defects repair":
  - Reading it proposed 1 task, 2 project updates and 2 commitments, each quoting the email. It took about 20 seconds.
  - Reading it a second time was refused: "This email was read already".
  - Add to Tasks put the task in Tasks, due Friday 7 November, with the email as its reason.
  - People and the project's page showed "from the email “Block B – final defect list”".
  - Forget took the email's waiting proposals with it.
- Reviews:
  - The migration review found 0018 safe. It asked for the email's text to be stored last in the row and for one commitment per email and item; both were done before 0018 ran. It found 0019 safe as written.
  - The architecture review found three must-fixes, all fixed:
    - a crafted .eml could crash the app;
    - an answer the app couldn't read marked the email as read with nothing in it;
    - commitments and project updates from an email said they came from a meeting.
  - Its other points were taken too (see D-056). The main one: your tasks from an email now wait for you to add them, like every other proposal, because someone else wrote the email.
  - A second architecture review of that change found one must-fix, now fixed: a mail program's wrong weekday ("Thu, 30 Oct 2026") made the email's day today. Its other points were taken too: a proposal whose task you deleted no longer shows "In Tasks", and the buttons stay busy until the list is fresh.

## Release
- **v0.9.2** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the two new tables.

## Manual steps
1. The first time you use Read the email selected in Mail, allow New Dawn to control Mail when macOS asks. You can change this later in System Settings → Privacy & Security → Automation.

---

# Last Build — People, and what they promised you · release 0.9.1
_2026-10-30_

## What changed
- **People** (sidebar, D-055). New page:
  - first, **commitments waiting for you**: what someone said they would do in your meetings;
  - then everyone your meetings named, grouped by organisation, each with when you last met them and their open commitments.
- **Commitments come from your summaries.** After each summary, the action items that belong to someone else become commitments for you to confirm: who, what, and by when as said. Your own items still go to Tasks. Nothing extra runs: the summary already says who does what.
  - **Confirm** keeps one open on the person's page until you mark it **Kept** or **Dropped**. **Dismiss** sets aside one that isn't really a commitment.
  - **Look through earlier meetings** collects them from meetings summarised before this release.
- **A page per person:**
  - **Waiting for you**
  - **What they owe you**
  - the **Meetings** they attended
  - your **Notes**
  - **Kept and dropped**

  Names and spellings are still managed in Workspace → People.
- Deleting a meeting keeps the commitments you confirmed, as Tasks does. Deleting a person keeps their commitments under their name.
- Checks: 154 Rust tests pass. New ones cover:
  - a name finds one person or none: never a family name alone, never one of two namesakes;
  - statuses only move forward, or back by Undo.

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app:
  - Look through earlier meetings found 8 commitments in the two dev meetings.
  - Confirming Ms. Chi's put it under "What they owe you" on her page.
  - Her page lists the meeting that named her ("Chi" matched "Ms. Chi"), marked Involved.
  - Dismissing Mr. Duc's moved it to Dismissed, with Undo.
- Reviews:
  - The migration review asked that the database keep your confirmed commitments if a meeting is deleted, and that merging two people move their commitments and notes. Both were done before the migration ever ran.
  - The architecture review found no rule violations and three must-fixes, all fixed:
    - commitments by organisations vanished once confirmed;
    - commitments didn't follow name corrections;
    - the check for "is this item yours" differed from the one Tasks uses.
  - Its other points were taken too (see D-055).

## Release
- **v0.9.1** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new table.

## Manual steps
1. People → Look through earlier meetings, once, to collect the commitments from meetings summarised before this release.

---

# Last Build — Projects that keep themselves up to date · release 0.9.0
_2026-10-30_

## What changed
- **Projects** (sidebar, D-054). Add the projects you follow in a workspace: a name, a phase, the next milestone and its date. Each project has its own page:
  - **Updates waiting for you**
  - **Where it stands**
  - **Risks** and **Decisions**: add or remove items yourself
  - **Meetings**
  - **History**

  Every section shows, even before it has anything in it. Status is one click (Active, On hold, Done), and ⋯ → Edit… changes the rest.
- **The Projects agent proposes; you apply.** After each summary, in the background (when your Mac is free), New Dawn reads the summary against your open projects. It links the meeting to the projects it touched and proposes what changed: a milestone and its date, a phase, a status, a new risk, a decision or a note.
  - Each proposal quotes the sentence of the summary it rests on and links to the meeting. Apply puts the change into the project and its history; Dismiss sets it aside.
  - The sidebar shows how many updates are waiting.
- A proposal must hold up before you see it. New Dawn drops:
  - a project that doesn't exist;
  - a field it doesn't know;
  - a date that isn't a real date;
  - a quote that isn't really in the summary;
  - the same change twice.

  A moved milestone still comes through as a new proposal.
- **Find project updates** in a meeting's ⋯ menu runs the agent at once for that meeting.
- Deleting a meeting removes the proposals that quote it; what you applied stays in the project.
- The Projects agent appears in Workspace → Agents with its guide, its model, and how many of its proposals you applied.
- Checks: 152 Rust tests pass. New ones cover:
  - only proposals that hold up are kept;
  - statuses are read kindly ("paused" is On hold).

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app, on "Block B defects repair":
  - Find project updates on the 88-minute defects meeting took 39 seconds. It linked the meeting and proposed a decision ("INCONS pays for repairs if defects are confirmed…") quoting the summary.
  - Apply put it into Decisions and History, and the sidebar count cleared.
  - Running it again didn't propose the decision a second time.
- Reviews:
  - The migration review asked that deleting a meeting keep the updates you already decided. That was done before the migration ever ran.
  - The architecture review found no rule violations and three must-fixes, all fixed:
    - Delete project happened in one click and now asks first;
    - a milestone proposal without a date could erase your date;
    - the Projects agent's number didn't show.
  - Its 11 smaller points were taken too (see D-054).

## Release
- **v0.9.0** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new tables.

## Manual steps
1. Projects → New project… for each project you follow (the agent reads only open projects). From the next summary on, updates appear on their pages; for an earlier meeting, use ⋯ → Find project updates.

---

# Last Build — Search your meetings by meaning · release 0.8.5
_2026-10-30_

## What changed
- **Search by meaning** (D-053). Searching on the Meetings page now also shows **Related by meaning**: meetings close to what you typed that don't use your words, in Vietnamese or English. For example, "concrete core testing" finds the Vietnamese meeting about drilling concrete cores. Each result shows its closest passage and opens the transcript there. The page's filters apply to these results too.
- It needs the **bge-m3** model (1.2 GB): Settings → Models → Search by meaning → Download. Ollama fetches it. Text search works without it.
- Your meetings are indexed in the background, when your Mac is free, like the other catch-up work. After that, a changed transcript or summary is indexed again, but only the parts that changed.
- Embedding models no longer show in the text-model lists (they can't write summaries). Settings can remove bge-m3 again.
- When Ollama is busy writing a summary, a search by meaning may say so and give up after 20 seconds. The text results are always there.
- Checks: 150 Rust tests pass. New ones cover:
  - lines become pieces of a few lines, and a long line is cut;
  - the similarity measure is safe on odd vectors.

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app:
  - The two dev meetings were indexed in the background: 129 pieces of the 88-minute meeting in 7 seconds, 8 of the short one in 2.
  - "load-bearing capacity of the structure" matches no meeting by its words, but Related by meaning finds "Building defects review" at 59:36. That's the Vietnamese passage about drilling cores to test compressive strength.
  - "who pays to repair the cracks" finds the same meeting through its summary.
  - Settings → Models shows bge-m3 as installed, with Remove.
- Reviews:
  - The migration review asked for one combined index instead of two, and for the "checked" time to be taken before the pieces are read. Both were done before the migration ever ran.
  - The architecture review found no rule violations and two must-fixes, both fixed:
    - a meeting with nothing to index would have been retried every 30 minutes;
    - one meeting that failed would have stopped a workspace's indexing and left its Run now on.
  - Its other points were taken too: streaming search, busy vs not running, unloading the text model first, making way for your work, logging, Remove, and the page's states (see D-053).

## Release
- **v0.8.5** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new table.

## Manual steps
1. To search by meaning: Settings → Models → Search by meaning → Download (1.2 GB). Your meetings are indexed in the following half hour, when the Mac is free (or Run now in the ✦ list).

---

# Last Build — The agents learn from your edits, and you see how they're doing · release 0.8.4
_2026-10-30_

## What changed
- **What I learned** (Workspace → Agents, D-052). Once you've made five edits to the Summary agent's or the Tasks agent's work, New Dawn reads them in the background and proposes up to three lines for that agent's guide, each with its evidence ("You shortened the overview in 4 of 5 meetings").
  - **Approve** adds the line to what the agent is told, after its guide and winning over it. **Dismiss** sets it aside for good. Nothing changes until you approve.
  - Lines in use are listed under the proposals, each with **Remove**. They're kept apart from the guide text, so Reset to default doesn't lose them.
  - Approve waits while the guide editor has unsaved changes.
  - **Check now** reads your newest edits at once. Otherwise it runs when your Mac is free, like the other catch-up work.
  - It reads what you rewrote in the summary (sections, corrections, attendees) and in Tasks (tasks the agent added that you deleted or reworded, action items you sent to Tasks yourself).
- **A number per agent and model.** Each agent's card now says how its work fared with you, per model:
  - summaries kept without rewriting a section;
  - emails kept as first written;
  - names the check doubted, which you changed or kept;
  - tasks it added that you kept, reworded or deleted.
  A summary, its email or a task counts once it's three days old (time for you to read it), and only your edits since the AI last wrote it. Nothing from before 0.8.1 counts: that's when the app started keeping your edits.
- Checks: 148 Rust tests pass. New ones cover:
  - only lines with three decisions behind them, and not already known, are proposed;
  - several saves of one section count as one decision;
  - five new edits, or one a week old, make a reading due;
  - a task reworded and then deleted counts once;
  - approved lines follow the guide and survive Reset.

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app: from four Overview rewrites (plus one real edit), Check now proposed "Keep the overview to one sentence that leads with the outcome." in 20 seconds, citing the four. Approve put it in the agent's prompt. After the review's changes it shows under "In use" with Remove, and the guide itself stays the default. Checked in dark and light.
- Found on the way: with thinking on, the model used 6,000 tokens on 25 edits without answering. Without thinking it was undecided, and the default guide's "Be detailed" held it back. Asking for a note on each edit first, and saying learned lines win over the guide, gave the right line in three runs out of three.
- Reviews: migration 0014 (additive) was reviewed. Its one point was taken before release: deleting a meeting also removes proposals built on its edits. The architecture review found no rule violations and three must-fixes, all fixed:
  - Check now failed while a scheduled reading was waiting for a free Mac;
  - a meeting deleted during that wait could still feed a proposal;
  - Approve threw away unsaved edits in the guide editor.

  Its other points were taken too (see D-052).

## Release
- **v0.8.4** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new table.

## Manual steps
- None. Proposals appear by themselves after five edits to an agent's work; Workspace → Agents → Check now asks at once.

---

# Last Build — Your voice and theirs, kept apart · release 0.8.3
_2026-10-30_

## What changed
- **A discussion keeps your microphone and the call apart** (D-051). When you record with the call's audio, New Dawn now also saves the microphone and the call as two files beside the recording.
- **Transcripts say who spoke.** A discussion recorded this way is transcribed one side at a time: your lines are marked **You**, the call's **Them**, as Live does. When two people talk at once, both are kept. Summaries already use these labels to tell your lines from the others'.
- When your microphone picks up the call through the speakers, those copies are dropped (Live's check). Headphones still give the cleanest result.
- A meeting held in a room (no call) is transcribed as before: when the call side carries under 20 seconds of speech, no line is marked You or Them, so nobody else's words become yours.
- Imports, presentations and meetings recorded before 0.8.3 are transcribed from the one recording, as before.
- If one of the two side files can't be written (a full disk), it is dropped and the recording carries on; the transcript then uses the one recording.
- Recordings take more room: about 65 MB an hour instead of 22 (three compressed files). Deleting a meeting or the retention setting removes all three.
- **The speakers engine is chosen** for the next two releases (who spoke when, then who it is): sherpa-onnx with pyannote segmentation and TitaNet voice prints. On this week's 30-minute test meeting it ran at 6.5× real time, and the six largest groups it found held nearly all the talk; the many small leftovers will be merged into the nearest large group. FluidAudio, the faster option that runs on the Neural Engine, couldn't be tried, and sherpa-onnx couldn't be built into the app either: this unattended session isn't allowed to build code fetched from outside (see Manual steps). The next releases (What I learned, memory search, projects…) go first meanwhile.
- Checks: 144 Rust tests pass (new: merging the two sides drops the call your microphone heard and keeps two voices at once; a meeting in a room gets no speakers). Clippy, tsc and the localhost check are clean.
- Seen in the dev app: a 90-second discussion made from the dev meeting's audio (you, then the call with a quarter-volume copy leaking into your microphone, then you again) came out as You 0:00–0:26, Them 0:30–1:00, You from 1:00, with no copy of the call among your lines. Both side files were compressed, the saved places cleaned up, and the meeting page shows You / Them at each change of speaker. Transcribed again from the compressed side files, the call's lines and your later lines came out right, but your first 26 seconds were missing: with the turbo model (the dev app's setting), that stretch came out as "Chào mừng các bạn đã theo dõi", a sign-off Whisper invents and New Dawn drops (D-047). Large-v3, the default for meetings, wrote the real words from the same file.
- Review: the architecture review found no rule violations and two must-fixes, both fixed. A meeting in a room would have marked every line as yours (the Mac's sound is recorded even without a call). A side file that couldn't be written would have stopped the whole recording. Its six other points were taken too (see D-051).

## Release
- **v0.8.3** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready.

## Manual steps
1. Nothing to set up: your next recorded discussion with call audio is split automatically.
2. **To get speakers (who spoke when, then who it is)**: allow Claude Code to build sherpa-onnx into the app (the `sherpa-rs` crate downloads sherpa-onnx's prebuilt libraries from GitHub while building) — a permission rule for that build, or approve it in an attended session. Optionally also FluidAudio (cloned to the dev data folder's `eval/fluid`), to compare the two.

---

# Last Build — Try a model on your own meetings before you switch · release 0.8.2
_2026-10-30_

## What changed
- **Reference meetings.** In a meeting's ⋯ menu, **Use as a reference**: its corrected transcript and edited summary become what other models are compared with (a Reference chip shows it).
- **Try a model on your meetings** (Settings → Models, D-050):
  - **Speech**: try another downloaded Whisper model against the one your meetings use, on two 90-second stretches of each reference meeting.
  - **Summaries**: try another text model against the one writing your summaries, on your summarised reference meetings.
  - You get figures (how close to your corrected transcript or edited summary, how fast, repeated lines, attendees and action items found), then **blind pairs**: the same stretch or section from both models as A and B. Pick the better one, or "about the same". When the new model wins, one button switches to it.
  - Nothing a check writes reaches your meetings.
- **Catch-up work waits for a free Mac.** A check runs when you've been away from the Mac for 5 minutes, it's charged (not on battery below 30 %), cool and has free memory. The ✦ list beside Record says why it waits, with **Run now**. Settings → Models → Catch-up work: "When I'm away" (default) or "Also while I work".
- **Text models are asked only what they can do**: a model that can't "think" is never asked to, and none is given more context than it has.
- Checks: 142 Rust tests pass (new: the catch-up gate's reasons, battery reading, the word agreement measure, blind pairs that are stable and go both ways, the windows heard, which errors stop a check). Clippy, tsc and the localhost check are clean.
- Seen in the dev app: a speech check (large-v3 against turbo on the 88-minute meeting: words as in the corrected transcript 75 % vs 77 %, speed 3.6× vs 8.7× real time; two blind pairs judged, tallied), a summary check where qwen3.5:4b couldn't write a readable summary of that meeting (shown as such), Try refusing a reference summarised before 0.8.1, the Reference chip and menu item.
- Reviews: migration 0013 was reworked before it was ever applied (no fixed lists of kinds, a deleted meeting takes its checks with it). The architecture review found no rule violations and three must-fixes — Cancel ignored while a check waited for the Mac; old summaries compared with your own edits; a meeting's title left in a check after deletion — all fixed, with most of its other points (see D-050).

## Release
- **v0.8.2** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new column and table.

## Manual steps
1. Mark two or three meetings you know well as references (⋯ → Use as a reference), ideally ones whose transcript and summary you've corrected.
2. To compare speech models, download another one in Settings → Models first (e.g. large-v3-turbo).

---

# Last Build — The app keeps what you correct, and summaries learn from your edits · release 0.8.1
_2026-10-30_

## What changed
- **Every correction you make to the AI's work is now kept** (D-049): a transcript line you fix, a vocabulary correction you undo, a correction you write, a summary section you rewrite, attendees you change, a name you confirm or keep, an action item you send to Tasks, an AI task you reword or delete, an email you rewrite. Text only, in the workspace, deleted with its meeting.
- **Summaries learn how you write.** When you've rewritten sections of earlier summaries of the same meeting type, the newest (up to three) go with each new summary as examples of your length, tone and what you include — never as facts. Only your own rewrites count (not the Corrections agent's), and only from now on: edits made before 0.8.1 weren't recorded.
- **The AI's own version is kept beside your edits**, so later releases can compare them (and count how often each agent gets it right).
- Workspace → Agents shows how many of your edits and decisions each agent has.
- Checks: 136 Rust tests pass (new: an edit reaches the ledger once, and becomes an example for the next summary of the same kind). Clippy, tsc and the localhost check are clean. Seen in the dev app: a section edit recorded, and the count in Workspace → Agents.
- Reviews: migration 0012 (additive) reviewed SAFE; its comments were corrected before it was ever applied. The architecture review found no rule violations and three must-fixes, all about recording only what really happened: saving a section unchanged no longer counts as an edit; name and attendee decisions are recorded only once saved. Its other points were taken too (examples fenced and only yours; the prompt budget follows the real prompt).

## Release
- **v0.8.1** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready; opening a workspace adds the new table.

## Manual steps
- None.

---

# Last Build — AI work that makes way for you, survives a quit and goes easy while you work · release 0.8.0
_2026-10-30_

## What changed
- **What you're waiting for goes first** (D-048). A transcription or a summary now pauses at its next step when you ask the AI for something quick (Corrections, Rewrite email, translating a slogan), then goes on.
  - Measured on the 88-minute meeting: a request made mid-transcription started 1.6 seconds later instead of after the whole transcription (about 9 minutes).
- **A quit loses nothing.** Transcriptions save their place about every minute, summaries after each part. Quit (or a crash) and reopen: the job goes on from there.
  - Measured: a transcription stopped at stretch 22 of 242 went on at stretch 22; a summary stopped after its first part went on at part 2 of 4.
  - Cancel still means stop: nothing is resumed after a cancel.
- **Gentle while you work** (Settings → Models → While you work; Gentle is the default). While you're using the Mac, on battery or when it runs hot, Whisper uses half the processor at a lower priority, and the text model leaves memory a minute after a job instead of five. Full speed is one click away.
- **See what the AI is doing.** While anything runs, a ✦ capsule beside Record names it ("Transcribing", "+1" waiting). Click it for the list, with progress and Cancel.
- Wording: a job waiting its turn now says "Waiting for another AI job"; Cancel on a waiting job says "Stopping…" until its turn comes.
- **A long summary no longer fails on one odd list.** Testing this release, part 2 of the 88-minute meeting came back twice with one list in the wrong shape, and the whole summary failed. Now the summary keeps everything else and leaves that list out.
- Developer builds show workspace logos and play recordings again (their data folder wasn't allowed since 0.7.6).
- Checks: 135 Rust tests pass (new: the lanes, a cancelled wait, the checkpoint's same-run rule, going easy, an odd list left out; and, with real Whisper, a batch that stops after one stretch and goes on). Clippy, tsc and the localhost check are clean. Seen in the running dev app, as listed above.
- Review: the architecture review found no rule violations and one must-fix, a rare way for the queue to freeze after a cancelled wait; fixed and tested. Its 14 smaller points were taken too (see D-048).

## Release
- **v0.8.0** is published on GitHub. **Your installed New Dawn wasn't touched**: update it from Settings → About → Check for updates when you're ready.

## Manual steps
- None.

---

# Last Build — A Whisper model per job, cleaner transcripts · release 0.7.8
_2026-10-30_

The first release of plan v3 (`docs/plan.md`).

## What changed
- **Meetings and Live each have their own speech model** (D-047). Settings → Models → **Used for**:
  - **Meetings** (transcripts after Stop, and imports): Automatic picks **large-v3**, the most accurate (it beat turbo 17–7 judged blind this week).
  - **Live**: Automatic picks **turbo**, twice as fast.
  - Or pick a downloaded model for either. Before, downloading turbo for Live also switched your meeting transcripts to it.
  - Your installed app has large-v3 only, so nothing changes until you download turbo: then Live gets faster and meetings stay on large-v3.
- **Cleaner transcripts**:
  - When Whisper repeats the same line three or more times in a row (it does over silence or noise), one copy is kept. Two in a row stay ("Dạ." "Dạ." is real).
  - YouTube-style sign-offs Whisper learnt from subtitles ("Hãy đăng ký kênh…", "Cảm ơn các bạn đã theo dõi") are removed sentence by sentence, so real speech in the same line stays. Only the sign-off forms are matched: "lượt đăng ký kênh tăng 15 %" in a sales meeting stays.
  - On this week's six test transcripts: 27 looped lines removed (17 from "Như vậy, lập tức." ×18), and both sign-offs cut.
- **Developer tooling** (debug builds only): the dev app can open a page, take dark or light appearance and be scrolled or clicked through a script file, without taking over the screen. Release builds don't contain it.
- Checks: 129 Rust tests pass (new: the model per job, looped lines, sign-offs); clippy, tsc and the localhost check are clean. Seen in the running dev app, light and dark: the Used for rows, choosing a model, back to Automatic.
- Review: the architecture review found no rule violations. Its fixes were taken: only the call-to-action forms of the sign-offs are matched, and a Live line that only lost a sign-off is no longer marked Unclear.

## Release
- **v0.7.8** is published on GitHub. **Your installed New Dawn wasn't touched**: update it from Settings → About → Check for updates when you're ready.

## Manual steps
1. Optional: download **Whisper large-v3-turbo** in Settings → Models (1.5 GB). Live then uses it; meetings stay on large-v3.

---

# Last Build — Transcripts in each speaker's own language, English summaries, summaries in one more language · release 0.7.7
_2026-10-29_

## What changed
- **The transcript keeps each voice's language** (D-046). In an "Auto" meeting, Whisper now chooses only between Vietnamese and English, separately for each stretch of speech.
  - Before, it chose among 99 languages. Your installed model (large-v3), when it guessed wrong, *translated*: English came out as stiff Vietnamese, and once as Malay.
  - When the choice is close, it writes the stretch both ways and keeps the real words.
  - Each line shows a small VI / EN tag. There is no translation column in the transcript.
  - Measured on this morning's meeting with your large-v3, every stretch written both ways: the right language almost everywhere, where the original transcript had more than ten translated stretches and a Malay line.
- **Summaries are always written in English** (with the email and the minutes), from the transcript as spoken.
  - **Translating the transcript into English first, as you suggested, was built and tested, and isn't in this release.** On this morning's mostly-English meeting it scored 6 against 5, but on the 88-minute Vietnamese meeting it lost badly, 3 against 6. Apple translating misheard Vietnamese line by line produced nonsense that the summary then repeated: "chùa Tứ Cụ" became "Tu Cu Pagoda", and the attendee list filled with words like "iPhone". It was also slower: 25 minutes against 18. Qwen reading the Vietnamese directly recovers more from context.
- **The summary in one more language.** The Summary tab now reads English | Vietnamese (or another language).
  - Pick the language: Vietnamese, French, German, Japanese, Korean, Chinese or Spanish. Nothing is translated until you press Translate.
  - The translation is kept with the meeting, and marked "The summary changed since" when the summary is re-run or edited (Translate again).
  - It's read-only: edit in English.
  - The default language is in Settings → General → Summaries → Second language (Vietnamese to start).
- **Translation uses Apple's translator, built into macOS.** It's on-device and free, and nothing leaves the Mac. It uses Apple Intelligence's model on your Mac.
  - Judged blind on your Vietnamese lines, it tied with Qwen 9B and was faster.
  - Google's API was ruled out: it costs past the free tier, needs a key and sends meeting text out.
  - Siri has no translation API.

- **Checks**:
  - 127 Rust tests pass. New ones cover the language choice, the translation gateway and summary translation end to end with Apple's translator on this Mac (French, names kept, checkboxes kept, out of date after a rename but not after a tick), and markdown handling.
  - Clippy, tsc and the localhost check (now covering the Swift helper) are clean. Migration 0011 was reviewed SAFE.
  - The architecture review found no rule violations. Its fixes were taken:
    - batch transcripts keep one row per line;
    - "out of date" follows the summary's words;
    - a hung translator is stopped;
    - an empty stretch can't win the language choice;
    - the helper's errors carry no meeting text.
  - The new Summary tab was seen in the running dev app with this morning's meeting.

## Release
- **v0.7.7** is published on GitHub. **Your installed New Dawn wasn't touched**: update it from Settings → About → Check for updates when you're ready.
- After updating, open this morning's meeting and choose **Transcribe again…**, then **Re-run** the summary.

## To think about (your call)
1. **A speaker who starts in Vietnamese and goes on in English within one stretch** (about 3 of 146 stretches in this morning's meeting): the English part still comes out translated, because Whisper picks the language from the start of the stretch (up to 25 s). Shorter stretches would fix it at about twice the transcription time.
2. **Exporting the minutes or the email in the second language**: the natural next step.

## Manual steps
1. Update the installed app (Settings → About → Check for updates).
2. **Recommended:** in System Settings → General → Language & Region → Translation Languages, download English and Vietnamese, plus any of French, German, Japanese, Korean, Chinese and Spanish you'll use. macOS then has offline models to fall back on when its Apple Intelligence model is unavailable.
3. On another Mac: translation needs macOS 26 with Apple Intelligence on, or the languages downloaded in System Settings → General → Language & Region → Translation Languages. Without them, Translate explains what's missing. Apple's model was unavailable once during testing, for about 15 minutes.

# Last Build — Safer corrections, summaries that survive slipped brackets, 4B vs 9B compared · release 0.7.6
_2026-10-29_

## What changed
- **The 4B-vs-9B comparison** (D-045). It ran the app's own agents on your two test recordings, and judges scored them without knowing which model wrote which.
  - 9B won all 9 comparisons. 4B is about twice as fast, but it invents facts and deadlines, marks discussed points "Not discussed", and reverses meaning when translating Vietnamese live.
  - **9B stays the model for everything.**
  - Live keeps 4B as its fast default, which is fine for English meetings. For Vietnamese meetings, set the Live agent to 9B in workspace Settings → Agents.
  - The full report is private, in the dev data folder: `~/Library/Application Support/co.majorisdigital.newdawn.dev/eval/report.md`.
- **Corrections only change what you asked for.** Both models sometimes turned one correction into changes nobody wanted. "Giang is the structural consultant" renamed five other people to Giang; a sentence was replaced with your correction's text. A change the AI proposes now stays only if:
  1. its wrong text is really in this meeting; a misheard spelling must be in the transcript;
  2. it is a spelling or a short fact, up to six words, unless you quoted it. Bigger changes come through the rewritten sections;
  3. it doesn't turn someone the meeting or your people list knows into someone else, unless you named them;
  4. an accents-only change goes to the spelling you wrote ("Hưng", never back to "Hung");
  5. each spelling has one right form.

  People your correction doesn't name are no longer saved to your people list. 4B had added five unasked, with changed titles and organisations.

  On the comparison's answers this removed every damaging change and kept the intended ones. A backwards fix where you write both spellings ("Soviko is Sovico") can still slip through; another correction ("Sovico, not Soviko") puts it right.
- **Summaries survive slipped brackets.** On long meetings the model sometimes breaks the JSON's brackets: a list left open, `}]` for `]}`, a cut-off ending. The app now mends that before asking the model again.
  - All three parts of the 88-min meeting that failed now read first time, saving about 2 minutes each.
  - An earlier run of that meeting had failed outright.
- **The names check accepts the plain list 4B answers**, so a 4B names check is no longer thrown away.
- **Developing never touches your installed app** (D-044). Debug builds and tests use their own folders (`….newdawn.dev`), and publishing no longer installs anything on this Mac.
- Tests: units for the guards (built from the comparison's real cases), the bracket repair and the names list; 123 Rust tests pass. Clippy, tsc and the checks are clean. The comparison tests (`eval_*`, ignored) are kept for the next model.
- Review: the architecture review found no rule violations and one must-fix. The repair could close an answer whose structure went wrong mid-way and save a partial summary. It now closes a missed `}` where it belongs and leaves an answer that ends inside a string alone. The review's smaller points were taken too: no meeting text in the names-check log, given names count, people your correction doesn't name stay out of memory, and the whole summary counts as "in the meeting".

## Release
- **v0.7.6** is published on GitHub (the .dmg, the signed update bundle and latest.json). **Your installed New Dawn wasn't touched.** Update it when you're ready: Settings → About → Check for updates.

## Manual steps
1. Optional: for Vietnamese live sessions, set Settings → Agents → Live to `qwen3.5:9b` and try one meeting. It needs about 7.7 GB beside Whisper, against 4.1 GB for 4B, and translates each line in about 6.5 s against 3 s.
2. Still pending: create the VIFC workspace, set your name, quit the app and say so. Your people and vocabulary will then be restored.

# Last Build — Live: you and the call apart, how clearly each line was heard, both ways, quick translation · task priority · release 0.7.5
_2026-10-26_

## What changed
- **Settings → Models**: one row per model — name, size, status — with its one action and a ⋯ menu (Update, Remove) on the right; rows line up whether or not they have an action. The Whisper rows the same.
- **All people**: a red ⊖ (out of the attendees) and a green ⊕ (into them) at the start of each row — Apple's list-editing controls; one click, focus stays with the person. Name and chips on the first line, the role under it.
- **Task priority**, like Reminders: ! / !! / !!! from the chip left of the due date (High, Medium, Low, None). Today and No date list the highest first; marks and dates line up in columns.
- **Status chips**: Transcribing and Summarising are yellow (Majoris yellow-100 / yellow-800). Warnings in light mode are a true yellow now too (the recorder banner, the start sheet's notice).
- **Live — start sheet**: Keep (Save as a meeting | Quick translation), Languages (Tiếng Việt → English | English → Tiếng Việt | Both ways), Format (Discussion | One presenter), remembered on this Mac.
- **Discussion**: your microphone and the call are transcribed apart, so two people talking at once become two clean lines; **You** / **Them** at each change of speaker. When your microphone only caught the call through the speakers, that copy is dropped (from 3 in a minute the page asks for headphones). Lines read in speaking order.
- **One presenter**: longer lines, read as paragraphs.
- **How clearly**: a line Whisper wasn't sure of is translated literally and marked "May be inaccurate"; one it couldn't make out (several voices, a weak signal) is marked **Unclear** and not translated — instead of a confident wrong translation.
- **Both ways**: each line's language is found as it's spoken (a VI / EN tag) and translated into the other.
- **Quick translation**: nothing is saved — no meeting, no recording, no summary. After Stop the lines stay on the Live page, with **Copy all** and **New session**, until you start another recording or quit — also when you stopped it from another page.
- Saved live meetings keep the labels and marks; the summary and the task finder know which lines were yours.
- **Translation fixes**: weekdays and ordinals are set from what was said ("thứ sáu" came out "Saturday", "Friday" came out "thứ Năm", "Thứ hai là…" came out "First is…"); English → Vietnamese no longer starts ordinary lines with "Vâng," or "Đúng rồi.".
- **Keeping up** (found in the running app): the speech detector runs on one thread (5× cheaper — on 8 threads it fell behind and the live text skipped 27 s in the first minute; file imports gain too), and a finished line no longer waits for its translation until people pause.
- New migration 0010 (additive: task priority; who spoke / how clearly / which language per line), reviewed SAFE.
- Reviews: each part had a correctness and a rules/UI review plus a fix pass; a final seven-lens review with adversarial verification confirmed 16 findings — 4 serious: your one-word answers dropped as "overheard", an answer saved before its question, days paired by position, English heard as Vietnamese and "translated" into English — all fixed; the architecture review of the last fixes found nothing. 119 Rust tests; clippy, tsc and the checks clean.
- **Seen in the running app** (dev build on your data, light and dark; your 13 s Vietnamese clip as the call and an English clip as your microphone): a quick session both ways — labels, VI / EN tags, translations both directions ("Friday" → "thứ Sáu"), then Stop → read-only with Copy all and New session; a saved session → the meeting page with the yellow **Summarising** chip, the summary (it credits your English lines to you) and two tasks from your lines (due Fri 30 Oct and 6 Nov); All people with ⊖; Tasks; Settings → Models. Once warm, lines appeared 2–6 s after each pause (Whisper 1.5–2.5 s, translation 1–2 s); the first line of a session takes about 10 s (the models warming up). Copy all was moved to the app's clipboard (like Copy as email) after the check: the browser's clipboard isn't there in every build. The test meeting and its two tasks were deleted afterwards.

## Release
- Published from the Mac as **v0.7.5** (the .dmg, the signed update bundle and latest.json); `main` is 0.7.5. Installed apps offer it in Settings → About → Check for updates.
- **Your data was cleared, as you asked — New Dawn on this Mac starts like a new user's** (onboarding, no workspace, no profile, no models): the app's folder (VIFC's meetings, recordings, brand, exports, tasks, people and vocabulary; your profile; the downloaded Whisper model), its browser storage (remembered choices), caches, logs and preferences went to the **Trash** — empty it to delete them for good, or use Put Back. Ollama's text models belong to Ollama and stay. Backups were off, so there were none elsewhere.
- Earlier the in-app checks had run migration 0010 on VIFC (the installed 0.7.3 then refused it); that was undone at your choice before the reset.
- **0.7.5 is installed in /Applications** (built here, the same as the release; the old 0.7.3 is in the Trash). Its first launch is a new user's: macOS asks again for the microphone and System Audio Recording.

## To think about (your call)
1. **Thresholds**: "Unclear" below 0.50 and "May be inaccurate" below 0.70 (Whisper's word confidence) are first guesses; clear test lines scored 0.78–0.98. Real calls will show whether they're too strict.
2. **You / Them**: everyone on the call is "Them" — telling them apart needs speaker recognition (deferred). In a hybrid room, everyone beside you is "You".
3. **Memory**: Ollama runs Qwen through MLX here; the 4B model grows from ~4 GB to 6–9 GB over a long session. With Whisper large-v3 beside it and your usual apps open, the Mac swapped 5–6 GB and lines slowed down. **Whisper large-v3-turbo** (Settings → Models, 1.5 GB) would make live lighter and faster.
4. **Quick translation lines** stay in memory until the next recording or quit — or should Stop clear them?
5. **Stop takes up to ~30 s in a busy session** ("Finishing…"): it transcribes and translates what was said before Stop. Shorter would lose the last lines.
6. **Weekdays in the summary**: the summary model read "Thứ hai là mình sẽ dựng…" as "on Monday" where the live translation said "Second". The weekday fix covers live translation only.

## Manual steps
1. Open New Dawn and go through the setup: allow the microphone and System Audio Recording, download the speech model (**Whisper large-v3-turbo**, the default — lighter and faster for live), create your workspace, then set your name in Settings → General (tasks and the minute-taker need it).
2. Discussion on a call: wear headphones (otherwise your microphone hears the call; the page tells you).
3. Empty the Trash to delete the old data for good (or Put Back what you want).

---

# Release pipeline on GitHub (no app changes) · release 0.7.4
_2026-10-26_

## What changed
- No app changes. 0.7.4 is the first release built by GitHub instead of the Mac, to prove the hardened path: the workflow runs only when started by hand from a version tag, waits for the owner's approval (the `release` environment, the only place the signing key is stored), and every action is pinned to an exact commit. A run started from a branch stops at once with a plain message.
- `main` now matches the released branch.

## Manual steps
1. GitHub → Actions → Release → Run workflow → "Use workflow from" → **Tags** → `v0.7.4` → Run workflow.
2. Approve it under **Review deployments** (about 10–15 min).
3. Don't also publish 0.7.4 from the Mac — one path per version.

---

# Tasks: your to-dos from meetings · release 0.7.3
_2026-10-26_

## What changed
- **Tasks** tab (right below Live): one list per workspace — Today (incl. overdue), Upcoming, No date, Completed. Type to add; set a due date from the chip; tick, edit, delete. The sidebar shows how many are due today or overdue.
- **From your meetings**: after a meeting's first summary, the action items that are yours become tasks — the ones assigned to you by name or title at once, and the unclear ones (Unassigned, teams, items that name you) decided by the new **Tasks agent** using your notes and the transcript. Each task links back to its meeting.
- **Add to Tasks** on every action item in a summary ("In Tasks" once added). Ticks stay in sync both ways.
- **Due dates** from what was said ("Friday", "thứ Sáu", "next week", "cuối tháng", "15/11"), counted from the meeting's date.
- New migration 0009 (additive: the task list), reviewed SAFE.
- Reviews: backend and UI each had a correctness and a rules/UI review plus a fix pass; a final cross-review (rules, a meeting end to end, edge cases) found 2 must-fixes — a tick in Tasks rewrote an edited Action items list; a namesake ("Lê Minh") could pass for you — and 7 improvements (renames reach Tasks, "lần thứ 3" isn't a weekday, older meetings aren't scanned on re-run, titles keep "<" and ">", …), all applied. 105 Rust tests. The Tasks agent, live on qwen3.5:9b: 12 of 12 runs picked exactly your items (the team item given to Minh's team stayed out).
- **Seen in the running app** on your data: the Tasks page (empty state with the "add your name" hint), and through the app's commands — Add to Tasks from a meeting (Ryan's item shows as "Ryan: …"), typed tasks due today / in 3 days / 2 days ago (Today lists the overdue one first), a tick, delete and restore (no duplicates). The test's tasks were deleted afterwards. The other screens couldn't be captured this time (macOS kept the test window in the Stage Manager strip while you were using the Mac); the UI was also driven in a browser with a stand-in for the app during the build.

## Manual steps
1. Set **Your name** in Settings → General — without it nothing is added automatically (the Tasks page says so).
2. Older summaries aren't scanned; use **Add to Tasks** on their items.

---

# Live: Vietnamese captions with English as people speak · release 0.7.2
_2026-10-26_

## What changed
- **Live** tab (right below Meetings; File → New Live Translation…, ⇧⌘R): start a live session, speak Vietnamese (or let the call speak) and the lines appear with their English beside them — about 2 s (Vietnamese) and 3–4 s (English) after each pause on this Mac. A−/A+ for text size; it follows the newest line unless you scroll up ("↓ Latest" brings you back). Pause and Stop are on the red pill as usual.
- The session is a normal meeting: the lines and their English are saved as they arrive, Stop skips the long re-transcription, and the summary runs automatically. The meeting's Transcript tab shows both columns.
- While live runs, the AI is reserved for it (other AI jobs wait for Stop); translation uses the fast `qwen3.5:4b` unless you pick another model for the **Live translation** agent in Workspace → Agents. If Ollama isn't running, live shows Vietnamese only.
- New migration 0008 (additive: `transcript_segment.translation`), reviewed SAFE.
- Reviews: backend and UI each had a correctness and a rules/UI review plus a fix pass; a final cross-review (rules, a whole session end to end, failure cases) found no blockers and 7 improvements, all applied — notably Stop never throws away the live lines (a session cut short keeps them, marks the gaps and skips the automatic summary).
- **Seen in the running app** (dev build on your data; your 13 s Vietnamese recording fed through the recorder 3 times): 9 lines, Vietnamese shown 2.3–4.2 s after each pause (Whisper large-v3 2.2–3.1 s), English 0.7–2.8 s later (qwen3.5:4b); "Thứ hai" → "Second", and your vocabulary turned "suy âm" into "siêu âm" ("ultrasound"). Stop → the meeting page with both columns and the player, then the automatic summary (written by qwen3.5:9b about a minute later). Screens checked: the Live page idle and live, the meeting's two-language transcript.
- Also: the summary no longer names people from your people memory that the meeting never mentions (seen in the test: "Marina Center, Keppel Land" in an overview).
- Two test meetings named **"Live test"** are in VIFC (the first from a cut-short test run) — delete them whenever you like.

## Manual steps
1. Use **headphones** for live sessions on a call: speaker sound feeds the microphone and doubles the lines.
2. Optional: download **large-v3-turbo** in Settings → Models for faster live lines — note it then also becomes the model for normal transcriptions.
3. Vietnamese letters such as ệ, ở, ữ come from the system font, because DM Sans has no Vietnamese glyphs — more visible at large live text sizes. Say if you want a font change (the project rules fix DM Sans).

---

# All people & attendees, your profile, a brand from logo + slogan + colour, minutes template made from it · release 0.7.1
_2026-10-26_

## What changed
- **All people / Attendees** (D-039): the summary's people card lists everyone the meeting involves; only **Attendees** go into the minutes. Remove / Add per person, Add all. Settings → General → **Everyone involved attends** (on by default) decides the starting point for new summaries; your choices survive re-runs. Old summaries: everyone stays an attendee.
- **Your profile**: Settings → General → **Your name** (app-wide); Workspace → Overview → **Your title** (per workspace). You're the first attendee of every new summary, the minute-taker in the minutes, the sign-off of the email, and Whisper/naming know your name.
- **Brand from three things** (D-040): the logo (the white version is made exactly — the VIFC white logo is reproduced pixel for pixel), the slogan in English (**Translate** with the new Translation agent; ~2 s), the primary colour (a document palette with contrast checks).
- **Minutes template made from the brand**: a neutral base learned from the uploaded VIFC template, recoloured, with your fonts and logo; preview drawn by Quick Look on the Brand page and in the wizard; **Update from brand** (Brand page) and **Reset template** (Meeting types). Workspaces with no template now get one automatically (before, export failed everywhere but VIFC).
- **The workspace wizard is real**: Workspace (name, slug, your title) → Brand → Minutes template → Finish. New workspace… in the switcher / menu opens it.
- Fixed: a long meeting could fail to summarise when the model echoed a done-flag; an SVG logo can't make the app read other files; the official VIFC white logo is never deleted.
- Reviews: each of the four parts had a correctness and a rules/UI review plus a fix pass; a final cross-part review (rules, integration, your real data) found 1 must (SVG file access) and 6 should-fixes, all applied. 85 Rust tests; clippy, tsc and the localhost check clean.
- **Seen in the running app** (dev build on your real data, dark and light): Settings → General (Profile, Meetings), Workspace → Overview (Your title), Brand (logo + white logo, Translate, colour, the Quick Look template preview), Agents (Translation), the meeting's All people card, and the three wizard steps. One layout bug found and fixed there: Remove dropped below a long job title. Translation checked against local Qwen: ~1–2 s per slogan once the model is loaded. Reduce Transparency not checked (a system-wide setting).

## Done on your data (you asked to align everything to the new layout)
- The registry got migration 0002 (`owner_title`). New Dawn 0.7.0 and older can no longer open this data — install 0.7.1.
- VIFC's minutes template was replaced with the updated VIFC starter (fills the minute-taker); the old copy, never edited, is in the Trash as "VIFC Weekly meeting template (before 0.7.1).docx".

## Manual steps
1. Settings → General → **Your name**. Workspace → Overview → **Your title**. (Until then the minutes show "[Full name — Title]".)
2. VIFC's header slogan is part of its own template ("…and Vietnam fintech"); `tokens.json` says "…Vietnamese fintech". Tell me which is official if it should change.
3. Word shows a substitute for **Be Vietnam Pro** until it's installed on this Mac (Playfair Display is installed). Install it from Google Fonts for exact minutes.
4. The two existing summaries: press **Rewrite** on their emails to get your sign-off; add yourself to their attendees (All people → Edit) or re-run them.

---

# One to-do list, meeting format & presenter, a model per agent · release 0.7.0
_2026-10-25_

## What changed
- **Action items = the to-do list** (Next steps merged in): "☐ **Owner** — task · due", tickable; the summary agent goes through every person named so no task is missed; "No task for …" lists present attendees without one. Done/Open in the minutes.
- **Attendees and task owners are checked against the transcript**: people only known from the people memory are no longer listed (live test: a 13-second clip went from 13 invented attendees to the presenter alone).
- **Meeting format**: Discussion or One presenter (+ name) in Import audio, Start recording, and on the meeting page (facts row) — before or after you stop. Whisper hears the presenter's name; the summary attributes statements accordingly. Migration 0007 (additive), reviewed SAFE.
- **A model per agent** in Workspace → Agents (Summary, Email, Corrections), with runs and average time per model; the automatic run stays on the Summary model. Only one text model stays loaded; a removed agent model falls back to the app's.
- **Ticks**: ticking a task isn't an edit, and ticks survive Re-run (either choice) and Corrections.
- Architect review: 3 must-fixes and the should-fixes applied (presenter cleared for discussions, model unloading, ticks, name check for short names like "Hà"/"Chi", presenter not duplicated as "Lan", time estimate per model, Agents select showing a missing model).
- Verified live through the app's commands: format set to a presentation by "Dr. Lan" → re-summarised → five sections, tasks in the new format, Dr. Lan first attendee; then only mentioned people kept. **Not seen on screen**: the dev window was shrunk to a thumbnail (Stage Manager) and can't be resized without granting the app a window permission — please look at the meeting header, the Action items list and Workspace → Agents.
- The test meeting "Nguyen Binh Khiem" was re-summarised twice for these tests (its format is back to Discussion).

---

# Transcribe again fixed, detailed summaries, to-do next steps, Corrections agent · release 0.6.0
_2026-10-25_

## What changed
- **Transcribe again works on summarised meetings** (it was refused: the check only allowed Recorded / Failed / Transcribed). It uses the vocabulary and people, then re-summarises keeping your edits. Verified live twice on the short test meeting.
- **More detailed summaries**: the summary may now be ~14k tokens long (was 6k); topics get 3–8 full sentences; the overview two or three paragraphs. A notes re-run that failed with "not the expected JSON" (a null where text was expected) can't fail that way any more — the reader repairs such answers.
- **Next steps = a to-do list**: tick a step and it's saved; ☐/☑ in the minutes.
- **Corrections** card under Attendees (sparkle): write what's wrong — "Duke is Đức – APD, Marina Center", "Wired Score → WiredScore", "go-live was 10 Oct, not 15" — and **Apply corrections**: the summary, email and transcript are fixed without a re-run (lines show Corrected + Undo), and the names/terms are remembered for the next transcript. A result line says what changed. Workspace → Agents lists the Corrections agent's guide. Verified live: "suy âm → siêu âm" fixed the transcript line, kept Whisper's text for Undo and taught the vocabulary (6 s).
- Architect review — must-fix applied: a malformed corrections answer is reported without quoting any meeting text (D-026). Should-fixes applied: the vocabulary learns only respellings from corrections (never facts like "15 Oct → 10 Oct"); transcript lines are corrected by exact spelling (≥ 3 letters, as for automatic corrections); section rewrites come before the renames; data is re-read after waiting for the AI queue; no "transcript changed" banner right after a correction; the corrections time-left ticks; a refused tick on a to-do shows its error; only top-level bullets become tasks; a missing `sections` still triggers the repair round.
- Note: the live tests changed the short test meeting "Nguyen Binh Khiem" (re-transcribed twice, now summarised; "siêu âm" in its transcript and vocabulary).

---

# Notes above the summary, both columns pinned · release 0.5.1
_2026-10-25_

- **Your notes** moved to the top of the summary column (above Highlights) and stay pinned while you scroll; long notes scroll inside the box (at most a quarter of the window) so the pinned card never covers the summary.
- The **right column** (Email, Attendees) is pinned as one block too; taller than the window, it scrolls on its own.
- Verified by screenshot at the top and scrolled.

---

# Vocabulary that steers Whisper, two-column summary, progress with time left · release 0.5.0
_2026-10-25_

## What changed
- **Your corrections now improve future transcripts** (D-036). Fix a name or term in a transcript line and it's learned (Workspace → **Vocabulary**: "WiredScore — misheard as Wired Score", Learned). Every new transcription (and Transcribe again) tells Whisper the meeting title, your people and your terms before it starts, then replaces the known mishearings in the result; those lines show **Corrected** (hover = what Whisper heard) with **Undo**, and the Transcript tab says how many lines were corrected. Confirmed names and attendee edits feed it through the people memory. Terms can be added and edited by hand; the tab shows exactly what Whisper is told.
- **Summary tab**: always two columns (wider 400 pt right column); **Your notes** first and pinned while you scroll the summary; Email and Attendees below it scroll. Window minimum 1180 pt (default 1360 × 820); the sidebar becomes the icon rail below 1280 pt.
- **Progress with time left** for every AI job: transcription ("Transcribing · 46 % · 1:12 · about 1 min left") and summary → names check → cover note, each with its own bar and estimate from the live token rate and past runs (`ai_log`); a sweeping bar until there's an estimate. Also on the Meetings list rows.
- Migration 0006 (vocabulary table, `transcript_segment.raw_text`), additive — migration-safety: SAFE.
- Architect review — must-fixes applied: transcript replacements match the **exact spelling** only (an alias like "Long" can't turn "a long time" into "a Hoàng Long time"; summaries still match any case), and capitalisation-only edits are never learned. Should-fixes applied: a learned pair must be a *respelling* (close once accents/spaces are folded — "Duke"→"Đức" yes, "the client"→"Acme" no); a hand edit clears the line's Undo; Undo is refused during transcription and points to Vocabulary to stop a bad correction; Whisper's originals are saved in the transcript's own transaction; lines where Whisper echoes its prompt ("People: … Terms: …") are dropped; group entries ("Richard and Harry") stay out of the prompt; the summary bar shows the stage while the model reads the prompt and its time left includes the names check and cover note; transcription time starts after the queue; Reduce Motion shows a faint full bar instead of a 30 % stub; the pinned card's shadow shows in dark mode.
- Not done: Rewrite email has no progress bar (it's a ~20 s call; the button says "Writing…"); a long meeting leaks ~700 bytes per speech segment through whisper-rs's `set_initial_prompt` (documented ceiling).

## Manual steps
1. Install 0.5.0 (Settings → About → Check for updates). The window opens wider.
2. In a transcript, fix a misheard name (e.g. "Duke" → "Đức") → Workspace → Vocabulary shows it as Learned → **Transcribe again** on a meeting → the line shows **Corrected**.
3. Write a summary and watch the progress card's time left.

---

# Agents: guides, highlights, naming check, cover-note email · release 0.4.0
_2026-10-25_

## Release 0.4.0
- Published from the local signed build (`scripts/release-local.sh`); the CI run fails at the key check until the `TAURI_SIGNING_PRIVATE_KEY` secret is set (see the 0.3.0 notes).
- Migrations 0004 and 0005 run on first open of each workspace (additive).

## Manual steps
1. Open a transcribed meeting → Write summary: watch the progress card go Writing → checking the names → writing the cover note; a notification "Summary ready · N names to confirm" if the naming agent has doubts; review them.
2. Type a note ("Vicky 1 go-live date; ThyssenKrupp warranty") → **Re-run with my notes** → the Highlights block appears first.
3. Email card → Preview (cover note with key points) → **Open in Mail** after exporting the minutes once: the DOCX is attached.
4. Workspace → Agents: edit the Summary guide (e.g. "Always name the chair"), Save, re-run; the agent's last run shows under its name.

## What changed
- **Agents** (`modules/agents.rs`, D-035): Summary · Naming · Email · Minutes, each with a Markdown guide — the bundled default (`src-tauri/agents/*.md`) until the workspace saves its own in `brand/agents/<id>.md` (atomic write; empty = back to default). **Workspace → Agents** shows each agent's description, the guide in an editor (Save / Discard / Reset to default, "Customised" chip) and its last run (model · status · tokens · seconds) from `ai_log.agent` (**migration 0004**, additive: `agent TEXT` + index). Ollama sessions are tagged with the agent (`TextSession::for_agent`).
- **Summary agent** uses its guide: the guide goes first in the system prompt, then the rules the app relies on (data fence, JSON shape, section list). New JSON field **`highlights`** (3–6 sentences from the owner's notes, each tied to what was said); shown first on the Summary tab in a tinted block, and in the minutes data as `highlights` (template field `{{#highlights}}{{text}}{{/highlights}}`).
- **Notes card**: "What you note here is highlighted in the summary" + **Re-run with my notes** (saves the notes, re-runs keeping edited sections).
- **Naming agent** (`modules/meetings/naming.rs`, migration 0005 `summary.naming_json`): runs right after every summary. Deterministic first — names in the summary that aren't in the people memory, or are one or two letters from a remembered name/spelling (also "Minh" ↔ "Đỗ Minh"), with the attendee's title/organisation and a transcript line as context — then one Qwen call with the naming guide may add suspects or improve a suggestion (if it fails, the deterministic list stands). The notification says "Summary ready · 3 names to confirm"; the Summary tab shows a card (heard as → suggestion …) with **Review names** → a sheet: per name, the spelling to use (editable), title, organisation, why, where it was said, **Use this / Keep as heard**; **Remember these people**; Apply renames across the summary (so the email and minutes follow) and teaches the memory — no AI run. Dismiss drops the list.
- **Attendees in place**: click a name in the Attendees card → name / title / organisation fields right there; Enter saves (renames everywhere, remembers), Esc cancels. The sheet stays for bulk edits.
- **Email agent** (`email.rs`): the email is now a **cover note** — greeting, "the minutes of … are attached", **Key points** (highlights first, then each discussion topic with its substance — more detail than the overview), Decisions, Action items, sign-off. Qwen writes it from the email guide right after the naming check; when the model isn't available the deterministic version (`render`, same shape) stands. **Rewrite** (sparkle) in the Email card writes it again from the summary as it is now; section edits no longer silently replace the email, name confirmations and attendee edits rename inside it (`people::rename_in_text`).
- **Open in Mail** now creates the message in Apple Mail with the **latest minutes export attached** (Apple Events via `osascript`, arguments not script text — `modules/meetings/mail.rs`); plain-text body; falls back to `mailto:` if Mail can't be driven. The command returns whether a file was attached.
- **Minutes**: no AI agent — the minutes are filled from the summary by rules and the Word template (D-024), so Workspace → Agents lists the three agents that talk to the model (Summary, Naming, Email). The `highlights` field is available to templates.
- Migration 0005 (`summary.naming_json`, additive) — reviewed with migration-safety like 0004.
- Architect review: no must-fixes; applied — the three agents share **one AI session** per run (the queue is held once; a queued transcription can't slot in and thrash Whisper/Qwen between them), `Naming` and `Email` progress stages ("Summary written · checking the names / writing the cover note"), status set to Summarised only after the agents (a DB hiccup there can't flip a saved summary back to Transcribed), Rewrite reports the real error, `open_in_mail` moved into the module (thin command), Email card says when the summary was edited since the email was written, Highlights show a ghost when notes exist but the summary predates them, nullable run duration.

---

# Breathing room, a real menu, one red Record, fixed Settings window · release 0.3.1
_2026-10-25_

## Release 0.3.1
- Bumped because the installed 0.3.0 can go blank (see "Blank windows" below); 0.3.1 reloads a terminated web content process. Published from the local signed build (CI still needs the `TAURI_SIGNING_PRIVATE_KEY` secret, see the 0.3.0 notes).
- Architect review: no must-fixes left — the sticky player strip uses `top: 0` (the panel is the scroller; its padding scrolls away), `--on-focus` token for menu text on the accent, the Settings panes use the global section classes (the duplicated ones in `Settings.module.css` are gone), `RecordIcon` deleted, the Workspaces form lets its inputs shrink so it fits the 500 px pane. Verified by screenshot: Meetings, meeting (summary with the ⋯ and Re-run menus open, transcript), Settings → Workspaces / Storage / Models at 780 × 580.

## What changed
- **Spacing** (user: "no spacing between elements, too compact"): content panel padding 32 → 40; sections 48 apart; cards 24 padding; grouped rows 16/24 with 56 min-height; field grids 20/32; page meta 12 below the title; tabs 40 above / 24 below; filters 40 below the header with 12 gaps; date groups 40 apart; the summary's two columns 48 apart, aside cards 24 apart; sheets 32 padding with 24 gaps and 12 between actions; Settings panes 40 padding, titles 32 below. All through the shared classes and tokens — no page got bespoke numbers.
- **⋯ menu** is now shaped like a macOS menu: 5 px inset, 24 px rows, 13 px text, 10 px side padding, hover/keyboard focus = the system accent fill with white text (what native menus use), hairline separators, 180 px minimum. Re-run ▾ uses the same.
- **One red Record control**: `.btnRecord` (red fill, white dot + label; red otherwise means destructive) for the toolbar button when idle and the dialog's Start recording. The sidebar Record item is gone — the toolbar button and ⌘R are the way in; while recording the toolbar shows dot · elapsed · meters · pause · ■ as before.
- **Settings window is fixed** at 780 × 580 and not resizable (System Settings), so the panes are laid out for one size.
- **Blank windows, found and fixed.** The grey windows seen in dev — and in the installed 0.3.0 (its content process was sitting at 7 MB, an empty respawn, with both its windows blank) — are WebKit terminating the web content process (memory pressure: Whisper and Qwen share 16 GB with it) and nothing reloading the page. The app now handles Tauri's `on_web_content_process_terminate` by logging and reloading that webview (`lib.rs`). Verified by killing the dev app's WebContent process: the window comes back by itself. The `CrashScreen` and the early error logger stay for the other kind of failure.

---

# Majoris design system on the Apple glass shell (parts 1–4) · release 0.3.0
_2026-10-25_

## Release 0.3.0
- Version 0.3.0 (package.json, tauri.conf.json, Cargo.toml), tag `v0.3.0`.
- **Why CI releases failed** (runs for v0.2.1 / v0.2.2): `failed to decode secret key … Missing comment in secret key` — the `TAURI_SIGNING_PRIVATE_KEY` secret on GitHub is not the key file's content. The file `~/.tauri/newdawn.key` is a single base64 line; Tauri wants exactly that. The workflow now checks the secret *first* (fails in seconds with the exact `gh secret set` command instead of after an 11-minute build), accepts the decoded two-line form by re-encoding it, and strips stray whitespace. Actions moved to Node-24 versions (`checkout@v5`, `setup-node@v5`).
- To make CI releases work: on this Mac run `gh secret set TAURI_SIGNING_PRIVATE_KEY --repo mjx-nvt/new-dawn < ~/.tauri/newdawn.key` and `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo mjx-nvt/new-dawn --body ""` (the key has no password). If the CI run still fails, `scripts/release-local.sh` publishes the same release from a local signed build.

## What changed — part 4
- **Fewer words.** The thirteen longest hints across Settings, Storage, Transcript, Summary and Attendees were cut to one plain sentence each (e.g. "After transcription, recordings are compressed to about 20 MB an hour."). Menus were already glass popovers and dialogs native sheets — nothing structural changed.
- **Never a blank window.** A root error boundary (`kernel/shell/CrashScreen.tsx`) shows the error and a Reload button if rendering throws; uncaught errors and unhandled rejections are logged to the console (the dev server mirrors it). Added after a wedged dev webview showed grey windows for a while — that turned out to be the dev process, not the code, but the safety net is cheap.
- **Dark-mode fix (real bug, found by screenshot)**: switching the Mac to dark while the app runs left the active sidebar item, the playing transcript row, orange chips and the update banner on their light cream background (WebKit never repaints a background whose value is `color-mix(…, transparent)` when the appearance changes at runtime; a fresh load in dark was fine). The six per-rule overrides are now three tokens flipped at `:root` (`--primary-tint`, `--primary-tint-soft`, `--on-primary-tint`) using rgba literals in dark; verified by sampling the active item's pixels through light → dark → light → dark.
- Architect review (part 4): no must-fixes; applied — the retention hint says again that the sweep is daily and counted from the summary's date; the search-hit highlight and the three status tints (chips, recording banner/dialog) became tokens too (`--status-*-tint`), so no static background uses `color-mix(…, transparent)` any more (the remaining ones — text selection, the sheet backdrop, the 55 % toolbar search/record pill, the 92 % canvas — either repaint on interaction or were verified to repaint on a flip).
- Sidebar section labels (APP · AI ON THIS MAC · DEVELOPER) use `--ink-2` at 600 like the other secondary labels. Over a dark desktop the glass sidebar is mid-grey and *all* dark text there is low-contrast (macOS's own vibrancy trade-off); in dark mode the labels read clearly.
- **Screens checked by screenshot**: Meetings, meeting (Transcript with player, Summary), Workspace (Overview, Brand, Meeting types, People), Welcome, New workspace, Settings (General, Workspaces, Backup, Storage, Models, PDF converters, About); the player, Storage and Meetings in dark mode. **Not checked**: Reduce Transparency (the accessibility default can't be set from a script here — please toggle it in System Settings → Accessibility → Display and look at a meeting page and the Settings window), and anything behind a click (sheets, menus, the switcher popover).

## What changed — part 3
- **Waveform player** replaces the default `<audio>` control on the Transcript tab: a glass strip with play/pause, the recording's waveform (click or drag to seek, ← → 5 s when focused), `0:00 / 46:05`, ±10 s buttons (Material `replay` and its mirror — Material has 10 s / 30 s glyphs, not 15, so 10 s it is), speed 1× / 1.5× / 2×. Played part in orange, the transcript line being played as a band behind the bars; the row highlight follows as before. Space plays/pauses and ← → skip 10 s anywhere on the tab unless you're typing or on a control.
- **Peaks from Rust**: `modules/meetings/waveform.rs` — 1200 max-|sample| values (0–255) over the file, decoded through the transcription loader on first open and cached as `waveform.json` beside the recording (the 46-minute import took a few seconds once). Command `get_waveform`; bridge `useWaveform`. A meeting without a recording (retention) shows no player, as before. Test: `peaks` covers the file and keeps the loudest sample.
- Architect review: no must-fixes; applied — a line under the strip when the recording can't be read or played, Space on the focused waveform, one ResizeObserver (not one per tick), no hex fallbacks in the canvas code, timestamp buttons drop focus after a click so Space pauses. Open: the strip keeps the old player's sticky offset (`top: -32px`); if its top looks clipped when stuck, set `top: 0` in `WaveformPlayer.module.css`.
- Verified by screenshot: the strip with bars, time, skips and speed on the 46-minute meeting. Not verified by me: actual playback and seeking (I can't click) — please press play, click into the waveform and try Space.

## What changed — part 2
- **One dropdown look.** Every `<select className="input">` is now the capsule the meeting-type picker had: 28 px, own chevron (two fixed greys as data URIs, light/dark), hover tint, full width only inside a `.field`. No new component — the native element already did the job; the per-page overrides (`MeetingsPage .select`, `MeetingPage .typePicker`) are gone. `<select>` now inherits the app font and colour (it was rendering in SF).
- **AI marker.** `SparkleIcon` (Material Symbols `auto_awesome`, inline — the only Material icon so far) plus `.btnAi`: the primary AI button is a blue-600 → orange gradient fill, other AI buttons get a 1 px gradient ring; `aria-busy` animates it while pending (the step button gives way to the progress card once the job runs). On: Import and transcribe, Transcribe, Transcribe again, Write summary (both places), Re-run ▾, Re-run and replace / Keep my edits, Re-run with corrected names. The summary head reads "✦ Written by qwen3.5:9b · 25 Oct 2026 at 10:55" (`.aiMark`). Tokens `--ai-from` / `--ai-to` / `--ai-gradient`; nothing else may use a gradient.
- Architect review: must-fixes applied (gradient survived hover/press; `<select>` font reset; the import sheet's button marked). Verified by screenshot: Meetings list filters, the summarised meeting (ring Re-run, written-by line, capsule type picker), the transcribed meeting (gradient Write summary).

## What changed — part 1
- **Typeface**: DM Sans (Majoris), bundled via `@fontsource-variable/dm-sans` (variable weight, optical size; `@fontsource/be-vietnam-pro` removed). Weights calmed: body 400, labels / buttons / chips / eyebrows 500, section titles 600, page titles 700.
- **Orange is the primary — only where navy was.** `tokens.css` now carries the Majoris orange ramp as `--primary` / `--primary-50…950` (the old `--brand*` variables were renamed; nothing new is tinted). Orange appears on: the primary button (500, hover 600, pressed 700), the active sidebar and Settings item (100 / 800), orange chips, eyebrows and links (`--primary-text` = 700 in light, 400 in dark), the progress bar, the playing transcript segment, the update banner. Neutrals moved to Majoris Slate (ink 900 / 600 / 500, surface-2 slate-100); status colours to green-700 / yellow-800 / red-600 (first steps that pass 4.5:1 as text on white; dark: the 400s).
- **Apple shapes**: buttons, segmented controls and the toolbar search are capsules; buttons press (`scale .97`); **secondary buttons are neutral grey** (Apple), no longer a tint of the primary. Cards, inputs, menus and sheets keep their radii.
- **The workspace brand is for documents only** (D-033). `applyBrand` is gone — `kernel/theme/brandLogo.ts` keeps only the logo for the workspace badge; `ContentRoot` and the Settings window set nothing from a pack. **Workspace → Brand** shows a paper-white first page (`DocumentPreview`: logo, heading in the heading font and primary colour, body font, slogan) and edits Logo · Colours (primary, secondary, accent, text) · Words and fonts. Corners / button / status / canvas controls were removed (UI concepts). The onboarding Brand step mock follows the same shape.
- Docs: CLAUDE.md (Stack → Typeface; Design rules rewritten), the `newdawn-ui-patterns` skill (design-system section, tokens, buttons, sidebar, pills), the architect-reviewer agent (DM Sans; no variable from a pack), README, `docs/plan.md` (D7 amended; brand-skins-the-UI line struck), D-033.
- Architect review: no must-fixes; should-fixes applied (stale README/comment/DECISIONS/plan lines, a commented fixed swatch, the dev mockup's hard-coded sizes, a note that `--primary-text` is for white surfaces only).
- Checks: tsc clean; localhost-only passes; no Rust change (no migration). Screenshots verified light + dark: Workspace page, Meetings list (orange capsule primary, status chips), Brand tab (VIFC-blue document preview inside an orange app), Settings → Models.

## Known gap (made visible by this change)
- The DOCX export takes its look from the meeting type's template file, not from `tokens.json`; the pack's colours/fonts/slogan are stored and previewed but not yet read by an exporter. They will be when the app generates documents itself (PDF / PPTX).

## Next (agreed plan)

## Manual steps
1. `pnpm install` (font package changed).
2. Run the app, look at Meetings, a meeting, Workspace → Brand, Settings; switch to dark mode; say what still feels "navy".

---

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
