# Last Build — Parity audit: plan v4 closed · release 0.12.0
_2026-11-03_

## What changed
- **docs/parity.md** (D-086), the living record of plan v4's rule (the same features and flows as Twenty and Plane, in our own code). It holds every one of the 99 feature rows of plan §3 (Twenty) and §4 (Plane), checked against the built app, with:
  - its mark: ✔ the same, ≈ ours with a difference, ✗ not built, ◌ later;
  - where it's built;
  - the exact difference.

  Plan §3–§4's marks now follow it (45 changed). The count:
  - 26 the same;
  - 3 the same with a part later;
  - 60 ours with a stated difference;
  - 8 not built, all as planned except Twenty's Time in stage and Plane's Create via a URL;
  - 2 for later.
- **Fixed from the audit:**
  - CRM: a Phones field's default country fills a new number; the **iFrame** widget can go on record pages; **Run workflow** on a table's selected records.
  - Projects: automations leave alone items in a module still under way; a work item's ⋯ has **Remove from <module>**; activity can be read **newest first**; a project's archived work items can be **searched**.
- **Written down for later**, in parity.md's "Known differences": a unique field made of parts compares each part on its own (a whole-value check was tried and set aside: it made the record engine's database worker panic in its test), exporting a selection, actor names on the timeline, Intake filters, more Analytics tabs, the Plane API's comments, and others.
- No migration. No new dependency.
- Seen in the dev app (light), on a check project "Parity check" with a module and an archived item:
  - the work item's ⋯ offered "Remove from Licensing", which took it out of the module;
  - "Newest first" put "You took it out of the module Licensing" on top;
  - the Archives tab's search for "room" left "Book the room";
  - the item view showed its "Proposed from meetings" section with "Nothing proposed from meetings";
  - in CRM → People, selecting a row showed "1 selected · Delete · Clear". The dev workspace has no manual workflow for People, so the Run workflow menu didn't show; it appears beside Delete when one exists.

  The check project was removed.
- Review: the architecture review found two must-fixes, both taken:
  1. The Run workflow menu would have opened under the table's sticky header; the selection bar now sits above it.
  2. A default country saved with every number made a unique Phones field refuse its second record. A phone's country and calling code are no longer compared on their own, and the default country fills only a number saved with neither.

  Taken too:
  - the activity order applies to the History and Transition tabs;
  - three rows marked ✔ that state a difference are now ≈, and the counts were redone;
  - the order is remembered with the app's own helpers;
  - menu items are keyed by position;
  - clearer labels.
- Checks: 219 Rust tests pass (new: a module under way holds its items from the automations, and a completed one doesn't). Clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — The Projects agent on Plane's model, Today · release 0.11.12
_2026-11-03_

## What changed
- **The Projects agent now proposes the work** (D-085). After a meeting's summary, besides the project's own updates, it proposes for each project the meeting touched:
  - new **work items** for the action items: title, assignee (you, or a person the summary names), dates, priority, labels, a parent, and the cycle running;
  - **state changes** for listed work items ("the report was sent" → Done);
  - **comments** on listed work items.

  Each rests on a sentence quoted from the summary.
- **Where they show:**
  - all of them in the project's "Updates waiting for you";
  - the new work items also above the **Work items** list;
  - what concerns one item, in that item's **Proposed from meetings** section.

  Apply makes the change as yours: a new item's description quotes the meeting, and a state change leaves the quote as a comment. Dismiss sets it aside.
- **Today** gains:
  - **Your work items**: assigned to you, due within a week or overdue;
  - **Cycles ending** within a week;
  - Intake's requests and the Inbox's unread, under Waiting for you.
- Migration **0043** (`project_update.item_id`, `payload_json`, a trigger that keeps decided proposals when a work item is deleted), reviewed by migration-safety (SAFE after one change). No new dependency.
- Seen in the dev app, on the dev workspace's "Block B defects repair" (BBDR) and its "Building defects review" meeting:
  - **Set-up:** the dev workspace took 0043. The check added a cycle "Week 45" running this week, a work item BBDR-2 "Send the November audit report to the contractor" (yours, due Thursday), and, for the check only, a line in the summary: "Ms. Chi confirmed the November audit report was sent to the contractor on 1 November."
  - **The run:** "Find project updates" ran on qwen3.5:9b in 56 s (3,479 tokens in, 637 out) and proposed 5 updates: a decision for the dossier, and four of the meeting's action items as new work items in Week 45, among them "Compile and issue the final Defect List … to the contractor" for Ms. Chi and "Verify the defect list against the current site conditions", each with its quoted line.
  - **Apply:** the first made BBDR-3 in Week 45, its description quoting the action item and "From the meeting “Building defects review”".
  - **What it looked like:** the project's Overview showed the cards (who, priority, cycle, quote, Apply / Dismiss); Work items showed the three still waiting above the list; Today listed BBDR-2 under Your work items (due Thu).
  - **Clean-up:** everything the check made was removed and the summary restored byte for byte.
  - **Not proposed:** the state change for BBDR-2 (see below).
- **"The report was sent" → Done**, the other half of the done-when, is not met in the app with this model. Four more runs on qwen3.5:9b (45–56 s each) proposed the action items every time, but never the state change:
  - two runs filed the line as a dossier note ("November audit report was sent to the contractor on 1 November.");
  - the others didn't use the line at all;
  - between them, three wordings of the rule were tried: states listed with their groups, "work done is its state change, not a note", and an example.

  So a safety net now does it in code (D-085): a sentence the model proposes for the project that says a listed open item's work was done also proposes that item's completed state, with that sentence. Unit tests cover the safety net, Apply (Done, plus a comment quoting the meeting) and the negative cases. Whether it fires in the app depends on the model picking the line up. The Projects agent's guide (Workspace → Agents) is your lever to tune it.
- Review: the architecture review found two must-fixes, both taken:
  1. One list the model gets wrong (null, a word, a single object) now costs that list, not the whole answer with its dossier updates.
  2. The same meeting read again is matched by the sentence a proposal rests on, so a reworded repeat isn't proposed again. A work item made once may come again from a later meeting.

  Should-fixes, all taken:
  - the prompt is bounded: 150 work items in all, the running cycle's and the most recently changed first;
  - names in the prompt are cleaned (one line, no markup, 80 characters);
  - only the people the summary names, plus you;
  - `<people>` is declared as data;
  - an empty project's first items are asked for;
  - Apply keeps only fields that still stand (labels, people, an open parent) and finds a state by name if it was made again;
  - a state proposal with no state is refused;
  - dates are checked as a work item checks them;
  - an item's view always has its Proposed from meetings section, and an archived project or item offers only Dismiss.

  Nits, taken:
  - the rollback only undoes its own claim;
  - Today reads project names without writing;
  - Today's waiting count leaves out archived projects, and its subtitle names Intake and the Inbox;
  - the proposals refresh every minute.

  Also: evidence quoted without the action list's `[ ]` box, and the answer read as its JSON object with `touched`, whatever comes around it. Left as is: Today's due list shows 20 without an "and N more".
- Checks: 218 Rust tests pass. Their new cases cover:
  - new items, state changes and comments that hold up and apply;
  - a list the model gets wrong;
  - the same meeting read again;
  - the safety net;
  - a deleted item's proposals;
  - the answer found inside other text.

  Clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Developers: Plane's API tokens, local REST API, webhooks · release 0.11.11
_2026-11-02_

## What changed
- **Plane's API on this Mac** (D-084): with the local API on, tools and scripts on this Mac can use Plane's v1 paths: `/api/v1/workspaces/<workspace>/projects/`, and each project's `issues/` (or `work-items/`), `states/`, `labels/`, `cycles/` and `modules/` (each with their work items) and `intake-issues/`. Answers come in Plane's JSON and pages. Requests send `X-API-Key`, and Plane's limit of 60 requests a minute applies (`X-RateLimit-Remaining`, `X-RateLimit-Reset`). A work item can also be read by its key: `…/issues/VIFC-12/`. The docs page (Open the docs) has a Projects section.
- **Workspace → Projects** (a new tab) holds Developers:
  - the Local API switch;
  - **API tokens**: a title, a description, and an expiry of a week, a month, three months, a year or never; shown once;
  - **Webhooks**: an address on this Mac and All events or Individual events (Projects, Work items, Cycles, Modules, Work item comments), with on/off, deliveries, a new secret and delete. Each change is posted as Plane's, signed with X-Plane-Signature.
- One key store: a token made here is also listed, and works, in Workspace → CRM → Developers, and the reverse.
- Migration **0042** (`api_key.description`, `webhook.format`), reviewed by migration-safety (SAFE). No new dependency.
- Seen in the dev app, light and dark. The dev workspace took 0042. Set-up: a new VIFC Fintech Hub; the Local API on; a token "Dev check script" ("Plane's API from curl", one week) made in the app; a Plane webhook to a listener on this Mac, all events. Results:
  - `curl -H "X-API-Key: …" http://localhost:50603/api/v1/workspaces/vifc/projects/` → 200, X-RateLimit-Remaining 59, and Plane's page (`count`, `next_cursor`, `total_results`, `results` …). The project read VIFC Fintech Hub, VIFC, network 0, cycles and modules on, intake off, with its default state.
  - POST `issues/` "Draft the rules" (high, an HTML description) → 201. PATCH through `work-items/` to urgent → 200. GET `issues/VIFC-1/` → Draft the rules, urgent, "<p>Ask Minh by Friday</p>", created by "Dev check script (API)".
  - No key → 401. Another workspace's path → 403 "This API token isn't for that workspace." An unknown item → 404.
  - The listener got `issue created` and `issue updated`, both with valid signatures. The update was priority High → Urgent; it is now sent in Plane's values, high → urgent. Both deliveries were logged with 200.
  - 60 more requests in the same minute → 429, Retry-After 35, X-RateLimit-Remaining 0.
  - Workspace → Projects showed the switch on, the token with its description, expiry and last use, and the webhook with All events.
  - Afterwards the project, token and webhook were removed and the Local API turned back off.
- Seen again after the review (scratchpad dev-0121b.out). A second change through the API (title and state) reached the listener as Plane's:
  - `name` "Draft the rules" → "Draft the rules, v2";
  - `state` Backlog → Done, with both states' ids;
  - the priority as `high` → `urgent`;
  - each delivery's work item with its state as an object, and `workspace_id` the workspace's id;
  - the webhook stored as `*` (All events).
  The window opened where it couldn't be captured, so the shots are from the first run; a probe read the panel's rows.
- Review: the architecture review found three must-fixes, all taken:
  1. Webhooks now name fields as Plane does (`name`, `start_date`, `target_date`, `estimate_point`) and send `null` for none.
  2. A description's autosaves no longer post to every webhook.
  3. Intake's PATCH checks the status and what it needs before writing anything, and the status a request already has is a no-op.

  Should-fixes, all taken:
  - Writes through the API are named by their token for projects, cycles, modules and deletes too.
  - A request waiting in Intake is no longer announced, nor read or listed through the work items' paths. Its Triage state isn't listed, and it can't be put into a cycle or a module.
  - Accepting a request announces a work item made.
  - The CSV import runs quietly.
  - Labels, assignees and modules come one event each, with their ids; cycles, parents and estimates carry ids.
  - A state or label made with doubled spaces is found again.
  - A large cursor can't overflow.
  - A rich description survives a round trip.
  - A webhook's work item is expanded, as Plane's.
  - The workspace's id is found through the hub.
  - The Projects module's own reads moved into it.

  Nits, taken:
  - paths without the trailing slash;
  - HEAD as GET;
  - whole months for automations;
  - numbered entities;
  - repeated ids once;
  - no "updated" for a change that changed nothing;
  - a project renamed from its dossier is announced;
  - a webhook can't point at the API itself;
  - "All events" as `*`;
  - caps on token names;
  - the docs list `work-items/<KEY>/` and the slug;
  - expired tokens marked, "Revoke" as the word, descriptions in the CRM list, "Reading…" while lists load;
  - no Projects tab with the module off.

  Left as they are:
  - one delivery task per event;
  - Plane's full user object and User-Agent;
  - Plane's five retries.
- Checks: 217 Rust tests pass. New tests cover:
  - Plane's paths answering as Plane, the cursor, null clearing, the key route, Intake's statuses checked before writing, a rich description kept, the Triage state hidden, the rate of 60, and entities;
  - a Plane delivery carrying the change, signed, with its work item expanded, plus a new secret.

  Clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Analytics, exports, CSV importer (Plane) · release 0.11.10
_2026-11-02_

## What changed
- **Analytics** (D-083, Plane's; Projects → Analytics in the sidebar): every open project or one — **Overview** (projects, work items, the open ones, active cycles, requests in Intake), **Work items** (Created vs Resolved day by day over the last 7, 30 or 90 days; the work by state group; Customized insights counted by state, state group, priority, assignee, label, cycle or module), **Cycles** and **Modules** (each one's progress). The choices are in the address.
- **Export** (the Work items tab): the work items your filters show as **CSV, XLSX or JSON**, with Plane's columns — Identifier, Name, Description, State Name and Group, Priority, Assignees, Labels, dates, Estimate, Cycle, Modules, Parent, Subscribers, Links, Comments, Created, Updated and Completed.
- **Import work items (Plane CSV)…** (a project's ⋯): a CSV as Plane exports it, or a workbook — the preview says how many rows and which of Plane's columns it has, then Import makes a work item a row: the state by name (made if the project lacks it), labels (made if missing), people by name, dates, estimate, cycle, modules, links, comments, and each item's parent by its identifier in the file. The summary says what didn't come in as written, row by row. Exports and imports are in the transfer history beside the CRM's.
- Spreadsheets: a cell that starts with =, +, - or @ is written after a ' so Excel and Numbers don't run it as a formula, and comes back without it on import (the CRM's exports too); a CSV opens in Excel with its accents.
- No migration (the transfer history of 0.10.8 holds them). No new dependency.
- Seen in the dev app (light and dark): VIFC Fintech Hub with VIFC-1 Draft the rules (a state of its own, In Review; label Legal), VIFC-2 Section 2, licences (its sub-item, with a comment "Ask Minh, by Friday") and VIFC-3 Book the room (Done). **Export ▸ CSV** wrote "3 work items saved as …/vifc.csv" with Plane's twenty columns. A second project, Sandbox copy: ⋯ **Import work items (Plane CSV)…**, the file, the preview, **Import** — "3 work items made", nothing to report; SBX-1 in In Review (made) with its label, SBX-2 under SBX-1 with its comment, SBX-3 Done. The history held one import and one export. **Analytics** for VIFC, Work items: Created vs Resolved over 27 Oct – 2 Nov showed today's three made and one resolved; By state group and the insights drew. Both projects and the history rows were then removed from the dev workspace.
- Seen again after the review: VIFC with a fourth item, "- follow up & check <rules>". **Export ▸ XLSX** and **▸ JSON** wrote 4 work items each — the workbook opened in Quick Look with Plane's twenty columns, every part of it strict XML; the JSON in Plane's column order. The workbook into Sandbox copy: the sheet's one primary action was **Choose a file…**, then **Import 4 work items**, the preview saying "The state "In Review" is made" and "The label "Legal" is made"; 4 made, SBX-2 under SBX-1 with "Ask Minh, by Friday", the fourth item's name intact. The same file again: "4 rows are named like work items the project has already — importing makes them again". Analytics over both projects counted by state: one Backlog, one In Review, one Done; Created vs Resolved showed today's 8 made and 2 resolved. All removed afterwards.
- Review: the architecture review found two must-fixes, both taken — an item the import had made was reported as skipped when its comment or subscriber failed (its children then lost their parent; now it stays made and the failure is said), and spreadsheet formulas in exported cells (guarded as above). Taken too: the preview checks every row as the import will (cycles, modules, estimates, dates, long names, parents not in the file, the states and labels it makes) and warns about rows already in the project; an archived project takes no import; the file is read off the app's main work and refused over 20 MB; problems capped at 500 with a count; the workbook's cells carry their references, are escaped as the minutes are and capped at Excel's 32,767 characters; JSON in Plane's column order; insights across projects one bar a name, states in their groups' order; the counts refresh after an import; the import sheet can't be closed mid-import and a slow preview of an earlier file doesn't land; Analytics says "Reading…" or "Couldn't be read" instead of zeros; Workspace → Data model's history names "Work items".
- Checks: 215 Rust tests pass (new: a CSV export imported back — states, labels, parents and comments, a "=1+1" name back as itself, the preview's made states and duplicate warning —, the workbook read back and the JSON; analytics by day and by property; a formula is text out and itself back in); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Home, Your work, Power K, navigation (Plane) · release 0.11.9
_2026-11-02_

## What changed
- **The sidebar's Projects section** (D-082, Plane's): Home, Your work, **Inbox** (its unread count), Drafts, Views, Active cycles, Archives, All projects (with the updates waiting for you), then each open project — ▸ opens it onto its Work items, Cycles, Modules, Views, Pages and Intake. **✎ Customize navigation** shows or hides each item. Today, Meetings, Live, Tasks, CRM, Email and Workspace stay at the top.
- **Home**: "Good evening, Tien" with the day, the date and the time; **Quick links** (an address and a title, opened in your browser), **Recents** (All, Work items, Pages, Projects — what you opened lately) and **Your stickies** (rich-text notes in six colours); **Manage widgets** turns each on or off and moves it up or down.
- **Your work**: Assigned, Created and Subscribed with their counts across your open projects; the chosen tab's work by state group and by priority, a pie of the state groups, your recent activity, and the items themselves (each opens in the peek).
- **Archives** (`g r`): what's archived across your projects — projects, work items, cycles, modules and pages — each restored there or opened where it lives.
- **⌘K** also finds work items (by key or title), projects, cycles, modules and pages, goes to Home, Your work, Inbox, Drafts, the views, Active cycles and Archives, and **Create a work item…** opens the create sheet. New keys: `g h` Home, `g w` Your work, `g i` Inbox, `g v` Views, `g r` Archives, `g a` Active cycles, `g j` All projects.
- Migration **0041** (`pm_sticky`, `pm_quick_link`, `pm_home_widget`, `pm_navigation`, `pm_recent`), reviewed by migration-safety (SAFE). No new dependency.
- Seen in the dev app (light and dark): the dev workspace took 0041; a new VIFC Fintech Hub with three work items and two unread notices. The sidebar's Projects section read Home · Your work · **Inbox 2** · Drafts · Views · Active cycles · Archives · All projects 3, then Block B defects repair and VIFC Fintech Hub. `g` then `r` opened **Archives** (Projects · 0, Work items · 0, …). With Book the room done and archived, Archives → Work items listed "VIFC-3 Book the room"; Restore brought it back. **Home** read "Good evening, Tien · Monday, 2 Nov 2026 · 21:22"; a quick link (vifc.gov.vn/sandbox, "Sandbox rules") and a sticky added by hand; Recents listed VIFC Fintech Hub, opened a minute before. **Your work** read Assigned · 3, Created · 3, Subscribed · 3 with its charts. **⌘K** "VIFC" offered the project. **✎** with Drafts unticked took Drafts out of the section. The project, the sticky, the link and the setting were then removed from the dev workspace. After the review, on the project's Cycles tab only the project and its Cycles part were current in the sidebar, and Home greeted "Tien Nguyen".
- Review: the architecture review found one must-fix, taken — every part of the open project lit up as current in the sidebar (now only the one on screen). Taken too: a visit is recorded each time something is opened, not by a cached query (Recents had kept an older order, and a window coming back to the front wrote visits); ⌘K shows no stale work items for one letter, and closes before Create a work item's sheet opens; "every widget is off" only once the widgets have loaded; the project on screen can be closed in the sidebar; a sticky has no Picture button and refuses a pasted picture, and deleting one asks first; the Recents tab is in the address; the ⌘K search returns the latest of each kind; Recents and Archives skip pages, cycles and modules of a project with that feature off; Your work's activity skips Intake requests; the go-to keys are read before a page's own; Archives says 500+ when it has that many work items; Home greets you by your whole name (a Vietnamese name's first word is the family's).
- Checks: 212 Rust tests pass (one new: stickies newest first in the six colours, words kept; a quick link's address once; the widgets on by default, reordered and one off; a navigation item hidden, an unknown one refused; recents newest first by kind; ⌘K finds a cycle; your activity); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Intake (Plane) · release 0.11.8
_2026-11-02_

## What changed
- **Intake** (D-081, Plane's): a project's **Intake** tab ("Intake · 3" while three wait) — requests waiting to become work. **Open** (pending) and **Closed** (accepted, declined, duplicates, and snoozed ones until their day), a sort (newest, oldest, priority, due date) and a priority filter; the requests on the left — ↑ ↓ or J K go through them — and the one chosen on the right.
- **A request** is numbered like a work item (VIFC-5) and has its title, description, priority, assignees, labels, due date, comments and activity, but shows only in Intake until it's accepted. While it's open: choose a state and **Accept** (it becomes a work item in that state — the project's default when none is chosen), **Decline**, **Snooze until** a day (it's back in Open on that day), or **Duplicate of…** a work item of the project, found by key or words (they're related as duplicates); **Delete** while it isn't accepted. **New request** (`N`) asks for a title, a description, a priority, assignees, labels and a due date.
- **From email**: when a project has Intake on, reading an email (Email → Read) also files the work the email asks of that project as a request — its title, the sentence quoted from the email, and the sender and subject; once per email.
- Migration **0040** (`pm_intake_item`), reviewed by migration-safety (SAFE; two optional checks taken before its first run: a snooze has its day, nothing duplicates itself). A project's Triage state is made the first time Intake is used. No new dependency.
- Seen in the dev app (light and dark): the dev workspace took 0040; a new VIFC Fintech Hub with Intake on and VIFC-1 Draft the rules. The tab read "Open · 0 · Closed · 0" and "No requests waiting — press N to add one". **New request** by hand: "Add the pilot list to the site", High — VIFC-2 in Triage, pending; two more by the app's call: VIFC-3 Translate the rules, VIFC-4 Draft the rules again — the tab read "Intake · 3". **Accept** VIFC-2 into Todo: VIFC-2, in Todo, accepted — and it appeared in Work items beside Draft the rules. **Snooze** VIFC-3 until tomorrow: "Intake · 1 · Open · 1 · Closed · 2"; with its day brought to today, it was back: "Intake · 2 · Open · 2". **Duplicate of…** "Draft" found VIFC-1: VIFC-4 a duplicate of VIFC-1, with the relation. The project and everything in it were then deleted from the dev workspace.
- Review: the architecture review found two must-fixes, both taken — a request can't be made a sub-item's parent (deleting it would have taken real work with it), and a snoozed request is in Closed until its day, so the Open count and the tab's agree. Taken too: a request opened from elsewhere (a relation, a notice, a window) says it's a request in Intake and links there; a closed request and its comments are read-only, in the app too; marking a duplicate is one change; making a request takes the lock before reading (two at once no longer fail); one primary button at a time; the duplicate search looks in the project; two actions at once can't both close a request; the activity says what it was (Pending or Snoozed); the email's sentence is one line; a test keeps the email prompt as it was when no project takes requests; the date of a request is this Mac's day; Snooze starts at tomorrow; an emptied title comes back; a read-only empty description says "Not yet captured".
- Checks: 211 Rust tests pass (one new: requests numbered from the start in Triage, kept out of lists and search and refused a state of their own; Accept into Todo, once, and not deleted after; Snooze refused for today, closed until its day, pending on it — the counts follow; a duplicate with its relation; Decline; Delete; filed from an email once a title; and the email reading keeps only requests the email makes); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Turn Intake on under a project's Settings → Features to use it (and to have emails file their requests there).

---

# Last Build — Pages (Plane) · release 0.11.7
_2026-11-02_

## What changed
- **Pages** (D-080, Plane's): a project's **Pages** tab — All, Public, Private and Archived, with search, a sort (last updated, last created, name), a label and Favorites as filters, favorites first; **New page** (`N` or `D`) opens a fresh page, Untitled until you name it.
- **A page**: its emoji and name, then the editor — type **`/`** for any block (text, headings 1–6, bulleted, numbered and to-do lists, quote, callouts — Info, Note, Warning, Success, Danger —, divider, table, code, a Mermaid diagram kept as code, picture, a sub-page, a work-item mention) and **`@`** to mention a work item; a mention opens its item in the peek. Text and background colours and a code block's language are in the editor's bar. Beside it: the **outline** (click a heading to go to it), labels, the pages inside it (New sub-page) and who made it.
- ⋯ **Lock** (the page then takes no edit, nor a restore), Make private / public, Duplicate, **Export as Markdown**, **Export as PDF…** (the Mac's print sheet, with Save as PDF; the page prints alone, in light colours), **Archive** (with its sub-pages; then Restore or Delete). **Versions** lists how the page was — the same hand's saves within ten minutes are one — and brings one back. **Full screen** gives the page the window (Escape leaves). A starred page is in the sidebar's Favorites.
- **Fixed** (since 0.11.1): a work item's description and its comments showed empty when the item was opened again (the editor read only a document stored as text, and the app hands it as an object). They show again, as do pages.
- Migration **0039** (`pm_page`, `pm_page_label`, `pm_page_version`; `favorite.pm_page_id`), reviewed by migration-safety (SAFE; a comment made exact before its first run). No new dependency.
- Seen in the dev app (light and dark): the dev workspace took 0039; a new VIFC Fintech Hub with VIFC-1 Draft the rules (with a description). The Pages tab read "No pages yet — press D to write one". **New page**, named Sandbox rules in its title; typed in the editor: "Ask @Draft" (the `@` list offered VIFC-1, Enter made the mention), then `/code` → "SELECT * FROM pilots;", three Enters out of it, `/table` → a 3 × 3 table with "Who" in its first cell. Saved and opened again: the mention, the code block and the table as typed. A click on the mention opened VIFC-1 in the peek, its description shown. **Lock** from ⋯: the editor and the name went read-only, and a write of the document straight to the app was refused ("The page is locked — unlock it to edit it"); the table stayed. Unlocked; the first version aged past ten minutes, the text replaced ("Replaced text") — two versions; **Versions** showed the older one (mention, code and table in its preview) and **Restore this version** brought it back: the page read "Ask @VIFC-1 Draft the rules / SELECT * FROM pilots; / Who" again, a third version. **Export as Markdown** wrote the title, the paragraph with "@VIFC-1 Draft the rules", the ``` code fence and the table. The project and everything in it were then deleted from the dev workspace.
- Review: the architecture review found six must-fixes, all taken — a document with no words but a sub-page link, a divider or a table is kept (it was saved as nothing; a work item's too); the work item's own version list kept its look (a style name was shared); a version's preview shows callouts, colours and sub-page links; deleting an archived page names its sub-pages, and an archived page lists them; a duplicate carries its pictures; a corner on a token. Taken too: opening the editor and leaving it without a change saves nothing (the page isn't "updated"); Markdown names sub-pages as they are now and keeps bold and italic around the words, not their spaces; a long page prints on as many sheets as it needs, without the Locked note; Escape that closes a menu, the link field or a peek doesn't leave full screen; Enter after a `/` with no match is the text's, and `/` in code is code; the Markdown file must be an absolute .md path; a mention is clickable from where it is written; Restore is off for the newest version (it is the page); a versions list that can't be read says so; a long emoji fits; deleting a project says its cycles, modules and pages go too.
- Checks: 209 Rust tests pass (two new: a page's versions — two saves of one hand within ten minutes are one, another hand's another —, a lock refusing its words and a restore but not its access, the restore of an older version as a version of its own, an index page of nothing but a sub-page's link kept, the same document twice no change, a sub-page archived with its page and deleted with it, a duplicate; and Markdown from every block); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Turn Pages on under a project's Settings → Features if it's off. Export as PDF… opens the Mac's print sheet: choose Save as PDF there.

---

# Last Build — Modules (Plane) · release 0.11.6
_2026-11-02_

## What changed
- **Modules** (D-079, Plane's): a project's **Modules** tab — **List**, **Gallery** and **Timeline**, with search, a sort (name, progress, start, target, newest) and filters (status, lead, member, a date range, Favorites, Archived too), favorites first; **New module** (`N` or `M`) with a name, a description, a status (Backlog, Planned, In progress, Paused, Completed, Cancelled), a start and a target date (either or both), a lead and members. A module's name is once a project, in any case.
- **A module opened**: its ★, its status, its dates, ⋯ Edit and Archive (completed or cancelled ones), Delete (its work items stay); its lead, members and progress; its work items — Add existing, New work item, × to take one out; **Needs attention** — the open items that are overdue or blocked by an open item, each marked Overdue or Blocked; the work by state group; Assignees, Labels, Priorities and States; **Links** (add, remove).
- **The timeline** draws each module from its start to its target date; drag a bar to move both dates, or an end to change one. It shares its axis and drag with the Work items timeline (now one component).
- Work items gain **Modules** — several at once: in their properties and the create sheet, as a display property, as a grouping on the List and the Board (an item sits in each of its modules' groups; drag one into another module), as a filter and in the bulk bar (Add to module). Each join and leave is in the item's activity.
- **Favorites**: a starred module or cycle is in the sidebar's Favorites and opens its project's tab on it; cycles starred in 0.11.5 move there; archiving one unstars it.
- Migration **0038** (`pm_module`, `pm_module_item`, `pm_module_member`, `pm_module_link`; `favorite.pm_cycle_id`, `favorite.pm_module_id`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: the copied stars get well-formed ids and skip archived cycles; then SAFE). Tried first on a copy of the dev database with three cycles — starred, plain, starred and archived: one favorite came of it, the first. No new dependency.
- Seen in the dev app (light and dark): the dev workspace took 0038; a new VIFC Fintech Hub with four work items — Draft the rules (due two days ago), Pick the pilots (due in nine days, blocked by Legal review), Book the room and Legal review. The Modules tab read "No modules yet — press M to make one" over its ghost rows, List · Gallery · Timeline above. **New module** by hand: Sandbox licence, In progress, 23 Oct – 22 Nov. Opened, **Add existing** with the first three ticked: three activity rows, and **Needs attention** listed Draft the rules (Overdue) and Pick the pilots (Blocked) — Book the room and the blocker not; "0 of 3 done · 0 %", Progress Backlog 3, the breakdowns 0/3. ★ put it in the sidebar's Favorites, opening the module. On the **Timeline** its bar, dragged 48 pt at Months, moved both dates three days (23 Oct → 26 Oct and 22 Nov → 25 Nov; a second drag to 29 Oct – 28 Nov). The Gallery showed its card, the List its row; Work items grouped by Module read "Sandbox licence" and "No module". After the shots: the search field had stretched across the toolbar (now 240 pt, the Cycles tab's too) and a gallery card centred its status and dates (now from the left). After the review: × on Book the room turned "Work items · 3 … 0 of 3 done" into "Work items · 2 … 0 of 2 done" at once, the list's row too. The project and everything in it were then deleted from the dev workspace.
- Review: the architecture review found four must-fixes, all taken — a module's numbers follow its items at once (an item taken out, made in it, marked Done, archived, deleted or added in bulk); unpinning a cycle or a module in the sidebar turns its ★ off in its tab; a module's links open in the browser through the opener, not in New Dawn; copying a work item that is in an archived module works (the copy joins only the open modules). Taken too: an archived module is read-only in the app, not only on screen — nothing joins or leaves it (its group gives up no item, its chip's ✕ is off), no link added or removed, no star; a member deleted from the CRM since doesn't block a module's edit; the Board's + in a module's column makes the item in it; the Table has a Modules column; with the Modules (or Cycles) feature off, a remembered grouping or property by it doesn't show; Needs attention keeps its order; a deleted module closes; an archived module's bar doesn't drag; an archived project's empty Modules tab doesn't offer M; the activity reads "added it to the module" / "took it out of the module". D-079 says a group dropped into moves the item, as Plane does.
- Checks: 207 Rust tests pass (one new: a module with three items — one overdue, one blocked by an open item — lists those two under Needs attention, then one, then none as they're done; names once a project in any case, dates in order, members once; another project's item refused; an item in two modules, out of one; the module filter; the activity rows; dates moved; a link once; ★ in the sidebar; archive completed only, unstarring it, read-only after — nothing joins or leaves, no link, no star; a copy joins the open modules only; delete leaving its items); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Turn Modules on under a project's Settings → Features if it's off.

---

# Last Build — Cycles (Plane) · release 0.11.5
_2026-11-02_

## What changed
- **Cycles** (D-078, Plane's): a project's **Cycles** tab — Active, Upcoming, Completed and Draft, with search, a date range and favorites first; **New cycle** (`N` or `C`) with a name, a description and both dates or none (a draft). A project's cycles never overlap.
- **A cycle opened**: Start now (an upcoming one) or Complete now (an active one); its progress ("2 of 5 done · 40 %"); its work items — Add existing, New work item, × to take one out; the **burn-down** (the open work each day against the dashed ideal line; points when your estimates are numbers) and the breakdowns by assignee, state and label. A completed cycle's open work can **move on** once to an active or upcoming cycle — it keeps its own numbers. Edit, Favorite, Archive (completed ones) and Delete (its work items stay, in no cycle).
- Work items gain **Cycle**: in their properties, as a display property, as a grouping on the List and the Board (drag an item into another cycle), as a filter and in the bulk bar; a quick add or a new item in a cycle's group goes into that cycle. An item is in one cycle at a time.
- **Active cycles** (Projects → Active cycles): the cycle every open project is running today, with its days left and its progress.
- Auto-archive no longer archives what an active cycle holds (Plane's rule).
- Migration **0037** (`pm_cycle`, `pm_cycle_item`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: days kept as YYYY-MM-DD; a transferred cycle's numbers kept as they stood; a deleted cycle's items get an activity row; a cycle completed early doesn't block the next one starting that day). No new dependency.
- Seen in the dev app (dark; the light run's window opened where it couldn't be captured — checked through the page and the database): a new VIFC Fintech Hub with Sprint 1 (26 Oct – 8 Nov, so in the middle of its run) and Sprint 2 (9 – 22 Nov); five work items in Sprint 1, two of them Done. The Cycles tab read Active · 1, Upcoming · 1, Completed · 0, Draft · 0, with Sprint 1 listed; opened, its burn-down said "3 work items open on 2 Nov, from 5" with its two lines, and "2 of 5 done". Complete now, then Move open work to… Sprint 2: Sprint 2 held the three open items, Sprint 1 said "Open work moved on" — the move gone from its bar — and still read "2 of 5 done · 40 %" (its numbers kept as they stood), with its two Done items, the burn-down and the breakdowns (You, Done, No label: 2/2). Active cycles then said "No cycle is running today" (Sprint 1 completed, Sprint 2 to come). After the review: the completed Sprint 1, edited, became "Sprint one" with its dates as they were, its description shown and no move offered (its work had moved on).
- Review: the architecture review found four must-fixes, all taken — a completed cycle can be edited (Edit always sent the disabled dates, so it always failed): its name and description change and its dates stay; a cycle's numbers follow its items at once (an item marked Done in its peek turned "1 of 3" into "2 of 3" in the dev app); a draft saved in a cycle that has since ended keeps its other fields and loses only the cycle; a new item moved to another project in the create sheet leaves the first project's cycle. Taken too: an item's cycle is written in the same transaction as the rest of its edit (and as the item made in it), and a cycle's Add existing and the move of its open work are one transaction each; a cycle completed early still blocks its own past days, only its last day is free; points when the project's active estimate is in points (an item without one weighs nothing); a moved-on cycle keeps its burn-down and breakdowns as they stood; archived items aren't counted; the automations leave alone what a cycle that hasn't ended holds — auto-close too; a completed cycle's group takes no drop and no quick add; with a project's Cycles feature off, no cycle shows anywhere (Active cycles included); the create sheet and the Table have Cycle; the cycles' date range and Archived too are in the address; an open cycle shows its description ("Not yet captured" when none); a cycle's dates are years 1900 to 2199, as items'; archived items can't be added; a deleted person counts as Someone; Move open work to… only when there is some; empty breakdowns show ghost rows; N and C wait while a cycle or a peek is open (and are listed with the keys); Remove from cycle in an item's menu; a one-day cycle says how much is open; items in a cycle not read yet keep their group. Left: a starred cycle isn't in the sidebar's Favorites yet — it comes with modules (0038).
- Checks: 206 Rust tests pass (one new: a two-week cycle in the middle of its run, the next one after it, a draft, overlap and half-dated cycles refused; five items in it, another project's refused, two Done — the burn-down 14 days long from 5 to 0, 5 open yesterday, 3 today, nothing for days to come; the breakdowns; the cycle filter; Complete now, then the three open items moved to the next cycle once, a second move refused, the move in each item's activity; a completed cycle taking nothing; archive completed only; an item taken out; delete leaving its items with an activity row; the moved-on cycle reading 5 and 2 done; a cycle starting today after one completed early; a completed cycle renamed with its dates as they are; the workspace's active cycles); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Turn Cycles on under a project's Settings → Features if it's off.

---

# Last Build — Filters and views (Plane) · release 0.11.4
_2026-11-02_

## What changed
- **Filters** (D-077, Plane's) under a project's Work items toolbar: Filters adds a rule — Title, State, State group, Priority, Assignees, Labels, Created by, Subscribed by, Mentions, Start date, Due date, Created on, Updated on — each a chip saying what it keeps ("Due date is this week"), opened to change it (any of / none of; is, is not, before, after, on or before, on or after, between, empty, not empty; days as dates or as today, tomorrow, this week, next month…, weeks starting on your first day of the week); Clear all. The filters are in the address, so a link keeps them.
- **Views**: Save as a view keeps the filters with the layout and display as they stand; a project's **Views** tab lists them (★ for the sidebar's Favorites, the layout and filter count, Private, Locked; Rename, Lock, Make it private, Duplicate as "Copy of …", Delete). Opening one shows its work items; change its filters or display and the bar offers Save view (not when locked), Save as a new view and Close view.
- **Workspace views**: Projects → Views — All work items, Assigned to me, Created by me and Subscribed, then your own — a spreadsheet of the work items of every open project, with the filter bar, Save view and Save as a new view. A starred view opens from the sidebar's Favorites.
- Migration **0036** (`pm_view`, `favorite.pm_view_id`), reviewed by migration-safety (SAFE; its hardening taken: the rules always a JSON array, a person deleted for good taken out of a view's people rules only, merged people becoming the kept one once, the calendar's span and the timeline's zoom kept with a view). No new dependency.
- Seen in the dev app (the window opened where it couldn't be captured, light and dark — checked through the page and the database): a new VIFC Fintech Hub with four work items — yours due Thursday, yours due Friday but Done, nobody's due Wednesday, yours due 10 December. Projects → Views opened on All work items with the four rows and the four built-in views. Filters → Assignees (you), Filters → Due date (this week, its default), Filters → State group, set to "is none of" Completed: the rules read exactly that. Save as a new view → "Mine this week" kept those three rules; ★ put it in the sidebar's Favorites; from Today, the sidebar's "Mine this week" opened the workspace's views page on it, listing VIFC-1 alone. The project's Views tab showed its two ghost rows (the workspace's view isn't the project's). After the review: from Today, the sidebar's "Mine this week" opened listing VIFC-1 alone; Save as a view on the project's Work items made "Board of the hub" and opened it at once under its bar; switching it to Board and Save view kept the board with the view, the project's own display untouched.
- Review: the architecture review found one must-fix, taken — Created on and Updated on filter by this Mac's day (they compared the UTC day: in Vietnam an item made at six in the morning counted as the day before). Taken too: a view just saved opens at once (the list is read again first), and Save view shows the saved view straight away; an open view's filters run only once the view is read (no flash of every item); an open view's working display belongs to that view as saved (switching views, or saving, starts from what it keeps); the title filter's words go to the filter on Enter or on leaving the field (Vietnamese is composed key by key); a locked view takes nothing but its unlocking; at most 50 filters and 200 values a filter, from a link too; a view's description is at most 2,000 characters; taking a view out of Favorites from the sidebar clears its ★; "A day…" starts on this Mac's day; an archived project's views can't be changed or saved from; deleting the open workspace view goes back to All, and a view that's gone says so; a filter value that isn't text is dropped; the spreadsheet's empty rows span its columns. Left: a view's pin is written by the views module rather than the CRM's favorites helper.
- Checks: 205 Rust tests pass (two new — the owner's weeks and months: Monday, Sunday and Saturday weeks, months across a year's end; and "Assigned to me, due this week, not Done" across two projects on a Wednesday — only the open items assigned to you and due that week; one project's; next week, a date range, no assignee, title words; an unknown field and a one-day "between" refused; saved as a workspace view in the table layout, starred and listed in the sidebar's Favorites; locked, only unlocking changes it; a copy "Copy of …", unlocked; merged people one, a deleted person taken out of the rules and the other rules kept; deleted with its favorite); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Layouts (Plane) · release 0.11.3
_2026-11-02_

## What changed
- **Five layouts** (D-076, Plane's) on a project's Work items: List · Board · Calendar · Table · Timeline. Each project remembers its layout, grouping, order and display — the properties shown (now with Attachments, Links, Created on and by, Updated on), sub-items, empty groups, the calendar's span, the timeline's zoom and the board's folded columns.
- **Board**: a column a group (state, priority, labels, assignees…), and rows when you sub-group (by assignee, say); drag a card to another column and row and it takes both at once — a new state and a new assignee in one change. Fold a column to a strip; + and a quick add in each cell.
- **Calendar**: a month or a week from your first day of the week, the work items on their due day; drag one to another day (its start moves with it); + on a day makes one due then; it says how many have no due date.
- **Table**: a row an item, a column a property; edit any cell where it is; the arrow keys move between cells, Enter goes in, Escape comes out; select rows for the bulk bar.
- **Timeline**: a bar from start to due date (a mark for one date) on a day axis — Weeks, Months or Quarters —, today's line; drag a bar to move both dates or an end to change one; the items without dates are listed below.
- Migration **0035** (`pm_display`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: the display's fields as columns, not one JSON blob; a person deleted for good leaves no folded board column, and merged people move theirs). No new dependency.
- Seen in the dev app (light; the Board in dark too): a new VIFC Fintech Hub with four work items — one yours, one Harry Do's, one from 10 to 14 Nov, one due 20 Nov. The Board sub-grouped by Assignees (kept with the project: board · state · assignees) showed the rows You, Harry Do and No assignee; VIFC-1 dragged from Backlog · You to Todo · Harry Do became Todo and Harry Do's in one change (its activity: state → Todo, assignees → Harry Do). On the Timeline, VIFC-3's bar dragged three days on read 13 → 17 Nov, and its peek showed the same dates. The Calendar showed November from Monday with today marked, VIFC-3 on the 17th, VIFC-4 on the 20th and "2 without a due date aren't shown". The Table listed the four rows with their pickers, and ArrowRight from the first cell moved to the next. After the review: the Board opened again as kept; VIFC-1 dropped on In Progress · No assignee became In Progress with nobody assigned; Escape during a Timeline drag left VIFC-3's dates as they were; the bars took their state's grey and today's line stayed orange.
- Review: the architecture review found two must-fixes, both taken — a row dragged in the List remembers the group it left (an item can sit in several: grouped by labels, moving it from B to C no longer takes off A instead of B); the Timeline's bars are no longer announced as sliders nobody can move — each row's name says its dates, and its peek's date fields are the keyboard path. Taken too: Sub-group by always offers None and never repeats the grouping (nor state with state group), here and in Rust; an item moved between label or assignee groups swaps the one it left for the target, even when it had both; Enter in a Table cell goes into its field, not a chip's ✕; the Table always shows the key that opens an item, and Escape in a cell cancels the edit; a new layout clears the selection; a refused display change reads the kept one back; the Timeline stays within a year of today either way (an item further away is listed below), and a date's year is 1900 to 2199; Escape, a lost pointer or leaving the window drops a Timeline drag; a new grouping unfolds the board's columns; Reset also clears the address's choices; nothing is drawn or changed until the items and the kept display are read; the Timeline's weeks start on your first day of the week, its bars take their state's colour (today's line is the one orange), and it redraws only when a drag passes a day; month names follow New Dawn's own. Left: the List's manual order is set by dragging only (as in Plane); the Table draws its pickers in every row (fine at a project's size).
- Checks: 203 Rust tests pass (one new: a project's display — the defaults when none is kept, a board sub-grouped by assignee with a folded column kept and read back, an unknown layout or property refused, a sub-group repeating the grouping stored as none, merged people moving their folded column, a person deleted for good taken out of it); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Your List's display from 0.11.1–0.11.2 (kept on this Mac) starts again from the defaults — pick its properties once more under Display.

---

# Last Build — Work items II and the Inbox (Plane) · release 0.11.2
_2026-11-02_

## What changed
- **Relations** (D-075, Plane's): a work item's properties gain Relates to, Blocked by, Blocking and Duplicate of — chips that open the related item, × to remove, + to find any work item of the workspace by its key (VIFC-12) or title words. Blocked by on one item reads Blocking on the other; two items relate once.
- **Links and attachments**: under the sub-items, Links (an address and an optional title — opened in your browser or Mail, copied, edited, removed; the same address once) and Attachments (Attach several files at once, open, Show in Finder, remove; copied into the workspace's folder, up to 5 MB each unless you change it in Settings → General → Work items). **Votes**: ▲ ▼ in the item's header.
- **Bulk changes**: a checkbox on each row and each group of the List; with items selected, a bar at the foot sets the state, priority, assignees, labels, dates or estimate (Update applies them), makes copies in a project, subscribes, archives (completed or cancelled ones only) or deletes. Every item is checked first: a refusal names the item and the rule, and nothing is changed.
- **Automations**: the project settings' auto-archive, auto-close and due-date reminders now run — once a day while New Dawn is open (a missed day catches up at launch), as "Automation" in the items' activity.
- **The Inbox** (sidebar, under Projects; ⌘K): what agents and automations do on the work items you follow, your mentions and due-date reminders — All and Mentions with their unread counts, Show (all notices, unread, snoozed, archived), Assigned to me · Created by me · Subscribed by me, Mark all as read; each notice can be marked read or unread, snoozed (1, 3 or 5 days, 1 or 2 weeks, or a date and time) or archived, and opens its work item beside the list. Settings → General → Inbox turns each kind on or off. Nothing is mailed or pushed.
- Also: an item's own window and a narrow peek show the item in one column, its title first (the properties no longer squeeze the description); you follow the items you change.
- Migration **0034** (`pm_relation`, `pm_link`, `pm_attachment`, `pm_vote`, `notification`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: a reminder always has its due date; an attachment's file is a name inside the item's folder, never a path; the same link once an item; reminders inserted with ON CONFLICT DO NOTHING). No new dependency.
- Seen in the dev app (light; the Inbox in dark too): in a new VIFC Fintech Hub with ten work items (one due tomorrow), the Backlog group's checkbox selected all ten ("10 selected"); Priority High → Update wrote ten priority activities and the bar closed. VIFC-1's properties showed Blocked by VIFC-2; a link typed as "sbv.gov.vn/sandbox" with the title "SBV sandbox decree" was kept as an https address and listed; ▲ read 1; Attachments showed its two ghost rows. Ninety seconds after launch the automations ran (the day noted) and the Inbox listed "Reminder · Due tomorrow — VIFC-1 Submit the sandbox application", with Inbox 1 in the sidebar; opening it marked it read and showed the item beside the list. The first look showed the peek's properties squeezing the title and description into a narrow column — in 0.11.1 too: the layout's width check measured the grid itself, which a container query can't — fixed; a narrow item now reads in one column, its title first. After the review: opening the reminder put its notice and its item in the address and highlighted only that notice; a file attached during the check (terms.pdf) was listed with Show in Finder and Remove, kept in the item's `files/pm/` folder.
- Review: the architecture review found two must-fixes, both taken — attaching several files checks them all first (one too large is named and none is attached; the command no longer loops, and a file can't stay attached unseen); a relation with an item archived since (the daily auto-archive) can be removed from the open side. Taken too: a copy no longer duplicates the original's attachments as unseen files; files are copied without holding the app up, and a file whose row fails is removed; the size message points to Settings → General → Work items; Mark all as read leaves the snoozed notices (they come back unread) and the kinds turned off; a related item of another project opens under its own project; a snooze time is kept in the app's own form; a bulk change takes only what the bar sets (not titles, descriptions, parents or order), each item once, one gone named by its id; the sidebar doesn't ask for the Inbox's count when Projects is off; a link or an attachment that can't be opened says so; the bulk bar counts only items still there; the Inbox highlights the notice opened (its id in the address) and says "Unread" to VoiceOver; deleting or archiving items refreshes the Inbox at once. Left: in the dev build, Open on an attachment is refused by the opener's folder scope (the release's folder is the one allowed) — as for the CRM's attachments.
- Checks: 202 Rust tests pass (one new: Work items II as Plane has them — ten items' priority changed in bulk writes ten activities, a backwards date range refused for the selection with the item named and nothing written, archive refused for open items, copies made; attaching checks every file first — one too large named and none attached; Blocked by read as Blocking on the other item, one relation a pair, search by key, unrelate; a link without its scheme taken as https, ftp refused, the same link twice refused; votes; the owner's own changes kept out of the Inbox, an agent's move to Done a Completed notice, muted kinds out of the counts, read and archived; an agent's comment naming the owner a mention and not also a comment; snoozed out of the Inbox and into Snoozed, a past time refused; the automations' reminder for an item due tomorrow, once; four months on, the idle open items cancelled by Automation and a Done one archived, and a relation with it still removable from the open item; once a day); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. The automations only act on projects whose Settings → Automations you turned on (due-date reminders are on by default).

---

# Last Build — Work items (Plane) · release 0.11.1
_2026-11-02_

## What changed
- **Work items** (D-074, Plane's): a project's **Work items** tab lists them as Plane's List layout — grouped by state (or state group, priority, labels, assignees, created by, none), ordered by hand (drag a row) or by created, updated, start, due or priority; **Display** picks what a row shows (ID, state, priority, assignees, labels, start, due, estimate, sub-items), whether sub-items and empty groups show (kept per project on this Mac). Dragging a row onto another state's or priority's group changes it. Each group ends with a quick add; `N` or `I` opens **New work item** (project, title, description, state, priority, assignees, labels, dates, estimate, parent; Create more).
- Each item is numbered in its project — **VIFC-1**, VIFC-2… (a number is never given twice) — and opens in a **peek** (side, modal or full screen), on its own page or in a window of its own: Subscribe; ⋯ with Copy ID, Copy as prompt, Make a copy, Copy to…, Open in a new window, Open the full page, Archive or Restore, Delete; the title; the description (pictures included) with reactions and its **versions** (a save within ten minutes of the last changes the same version; any can be restored); **sub-work items** with their progress ("1 of 2 done · 50 %") and an add; **Activity · Comments · History · Transition** (how long the item spent in each state); comments with one level of replies, @mentions, reactions, edit and delete; the properties at the side (Cycle and Modules come with 0.11.5 and 0.11.6).
- **Drafts** (Projects → Drafts): closing New work item with something written asks to keep it as a draft; a draft can be finished, copied, moved to another project or deleted.
- **Archives** tab: only a completed or cancelled item can be archived (read-only, still there to read, restore or delete). A state that items use can't be deleted — Settings → States says how many.
- Migration **0033** (`pm_item` and its assignees, labels and subscribers, `pm_comment`, `pm_reaction`, `pm_activity`, `pm_item_version`, `pm_mention`, `pm_draft`; `pm_project.last_sequence`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: an item's state is one of its own project's; a comment mentions a person once; a person deleted for good leaves no assignment, subscription, mention or reaction, and merged people move theirs; an estimate replaced or turned off clears the items' values). No new dependency.
- Seen in the dev app (light; the dark run opened where the window couldn't be captured): VIFC Fintech Hub's Work items showed the five state groups, each with its ghost row ("No work items yet — press N, or type one below") and its quick add. New work item → "Sandbox launch plan" → Cancel asked "Keep it as a draft?" → Save draft; Projects → Drafts listed it; its Create made **VIFC-1** in Backlog and the draft was gone. Two sub-items, VIFC-2 "Write the sandbox rules" and VIFC-3 "Pick the first fintechs" — one moved to Done — read "1 of 2 done · 50 %" under the description, and the list showed VIFC-1's "1/2 done" and VIFC-3 as "↳ Pick the first fintechs". A comment ("The rules go to the board on Friday.") took a 🎉 reaction. Moved to In Progress, the Transition tab read "Backlog — under a minute" then "In Progress — under a minute so far". Archive on it said "Only completed or cancelled work items can be archived" and nothing was archived. The peek showed the key, Subscribed, the title, the description editor, ☺︎+ and Versions, the sub-items' bar and the activity tabs, with the properties at the side (State, Priority, Assignees, Labels — "No labels in this project" —, Start, Due, Estimate, Parent, Cycle and Modules empty, Created, Updated); the full page showed VIFC-1 too. After the review: VIFC-2's peek opened from its address on Comments, with two ghost rows for its sub-items and two for its comments; History put `activity=history` in the address; Escape in the title kept the peek open; a due date typed was saved once, on leaving the field (one activity row).
- Review: the architecture review found six must-fixes, all taken — an item's own window has the app's permissions now (before, it couldn't be dragged, pick a picture or follow changes live); menu commands (Record, Import audio, New live session…) go to the main window only, never twice; restoring a version keeps the newest one (it used to overwrite a version under ten minutes old); a copy takes its pictures with it, so deleting the original leaves them; the activity tab is in the address (a peek or a page opens on Comments or History as left); empty sub-items, comments, versions and lists show ghost rows. Taken too: dates save when you leave the field (WebKit's date field changed at every digit — four saves for a year); Escape inside a field — the title, the @ list, a comment — no longer closes the peek; Escape or a click outside while asked "Keep it as a draft?" goes back to the form, and a draft you changed asks before closing; the description's autosaves are one activity row while they go on (ten minutes, as its versions); an archived item's comments and reactions are read-only in Rust too; Delete on the Archives page says the sub-items go with it; N and I don't fire over a sheet, while composing or after a `g` chord; a draft kept before the form grows still reads; merged people move inside drafts; Archive is off — saying why — until an item is completed or cancelled; Comment waits for words; label groups aren't list groups; the parent picker leaves out everything under the item; versions made in the same millisecond list newest first. Left: Plane's "Copy link" waits for a link New Dawn can open (`newdawn://`, plan §4.3) — Copy ID stands in.
- Checks: 201 Rust tests pass (two new: a work item's life as Plane has it — made from a draft as VIFC-1 in the default state with its maker following; two sub-items, one Done → 1 of 2; leaving Done clears when; the state changes kept for Transition; archiving refused in progress, allowed once done, then read-only and restored; a comment with a mention, a reaction taken back on a second press, a reaction on the description, one level of replies, an unknown emoji and an empty comment refused; two saves within ten minutes one version and one activity row, an agent's save a version of its own that a restore keeps; a state in use kept; numbers never given twice; delete taking its sub-items; a copy in another project in its default state without labels, its picture copied into its own folder and its description pointing there; date order, empty titles and parent loops refused; files kept and deleted with the item; and a draft kept before the form grew still reading); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Projects and their settings (Plane) · release 0.11.0
_2026-11-02_

## What changed
- **Projects is Plane's list** (D-073): cards — a cover band, the project's logo, its name and identifier, its description, lead and network, the updates its meetings brought — or a list; search, a network filter, an order (favorites first, name, newest, identifier), Archived; ★ to favorite, ⋯ for Settings and Archive. Projects you already had got an identifier from their name ("Block B defects repair" → BBDR) and Plane's five states.
- **New project** in two steps: a cover, a logo (an emoji), the name with its identifier (suggested from the name until you change it; one already taken is said at once), a description, Private or Public (a label — nothing leaves your Mac), the lead (you or someone in the CRM); then the features — Cycles, Modules, Views, Pages, Intake.
- **A project's page**: its logo, name and identifier, the dossier's status (Active, On hold, Done), and tabs — **Overview** (the updates its meetings brought, where it stands, risks, decisions, meetings, history — as before), Work items, Cycles, Modules, Views, Pages and Intake when they're on, Archives, **Settings**. The work-item tabs show their columns now; the next releases fill them.
- **Settings**: General (cover, logo, name, identifier, description, network, time zone; Archive, Delete), Members (lead, default assignee, default subscribers), Features, **States** (five groups — Backlog, Unstarted, Started, Completed, Cancelled — each keeping one state; add, rename, colour, describe, reorder, the default), **Labels** (one level of groups), **Estimates** (Linear, Fibonacci, Squares, T-shirt sizes, Easy to hard or your own, up to six values; one active, a new one archives it), **Automations** (auto-archive and auto-close after N months into a cancelled state, due-date reminders).
- Migration **0032** (`pm_project`, `pm_state`, `pm_label`, `pm_estimate`, `pm_estimate_point`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: the key NOT NULL; the close state and the active estimate cleared by their keys when deleted; one default state a project and label names unique, by index; a person deleted for good leaves no lead, assignee or subscriber; on/off columns checked; a project made whole in one transaction; names compared in full Unicode; Cycles and Modules on by default, as in Plane). No new dependency.
- Seen in the dev app (light; both dark runs opened where the window couldn't be captured): Projects showed Plane's cards — the project already there, "Block B defects repair", as BBDR with its "3 updates waiting", and "VIFC Fintech Hub" (made through the command) with its orange cover, its logo, its description and "Lead: You · Private". New project → "Sandbox launch" (SL suggested) → Next → Intake on → Create project opened its page: "Project · SL", tabs Overview · Work items · Cycles · Modules · Views · Pages · Intake · Archives · Settings, the Overview its dossier. A second New project with VIFC typed: the second step said "Another project uses VIFC — identifiers are unique" and nothing was made; the sheet now says "VIFC is VIFC Fintech Hub's identifier — identifiers are unique" on its first step and keeps Next off. Deleting Todo — the Unstarted group's only state — was refused and Todo stayed. Settings → States listed the five groups, each state's colour, name and description side by side, Default or Make default, Delete and "Add a state"; Estimates showed Fibonacci active (1 · 2 · 3 · 5 · 8 · 13) with Edit values and Turn off, and the new-system form (Linear … Custom). Everything made was deleted afterwards (the two projects; BBDR, the migration's own, stays).
- Review: the architecture review found two must-fixes, both taken — the General settings no longer redraw after each save (focus stayed lost and a stale name could be saved back: each field now keeps its own text, saved once on leaving it or on Enter, never while Vietnamese is still being typed); a label that isn't a group can go into one (and only a group's delete says its labels go too). Taken too: a project's edits are checked first and written in one transaction, only the fields given; the answer goes straight into the page's copy, and the controls wait while a change saves; colours are names of the app's palette (states and labels, drawn in light and dark), chosen once from a list — no more a save for every step of a colour drag; the dossier's changes (status, rename, delete) refresh the Plane side too; a subscriber now in Deleted records stays and doesn't block the others, a lead no longer in the CRM keeps its name in the list; the old dossier-only "New project" path is gone, so every project has its Plane side; an archived project is out of the Projects agent's reading and Today; covers are slate by default (the green and yellow ones, being status colours, are gone); the Plane side's errors show; Replace closes the values being edited; Delete is off — with why — on the default state and on a group's last; empty lists show two ghost rows; an identifier taken in a race is said plainly. Left: the colour lists after the review weren't seen — the window opened off screen twice; a time zone is checked for its shape (the list offers only real ones).
- Checks: 199 Rust tests pass (three new: identifiers from names; projects keeping unique identifiers, their states, settings in one write — a refused identifier writes nothing, not even the new name — a subscriber in Deleted records kept, archive; states, labels and estimates keeping their rules — the last of a group and the default, one level of label groups, names in full Unicode, up to six distinct values, one active system, values edited in place); clippy, tsc, the localhost check and the Tasks and meetings scripts are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Your projects keep their overview; their identifiers come from their names (Settings → General to change one).

---

# Last Build — The CRM agent, Today, formats · release 0.10.15
_2026-11-02_

## What changed
- **The CRM agent** (D-072, plan v4 §5.7): after each summary and each email you read, it proposes the companies, people (with their title and company), opportunities (with the stage the conversation is at, the amount and close date when said), notes and tasks for the CRM — each quoting the sentence that says so. They wait in the CRM's new **Waiting for you** tab (beside Commitments) and on the page of the record they belong to: what it would write, the quote, where it was read, **Apply** and **Dismiss**. Apply writes it into the CRM as the agent, once; a person at a company proposed beside it brings the company in too; a task already in Tasks is attached to the record rather than made twice. A meeting's ⋯ → **Find CRM proposals** reads its summary now. Workspace → Agents lists the CRM agent with its guide, model and numbers.
- **Today**: the CRM's proposals count under Waiting for you; a new **Opportunities to close** section — not won, closing within 30 days or past their date, soonest first; "This week's meetings" starts on your week's first day.
- **Formats** (Settings → General → Formats, Twenty's Experience settings): time zone, date (24 Oct 2026, 24/10/2026, 10/24/2026, 2026-10-24), time (24- or 12-hour), numbers (1,234.56, 1.234,56, 1 234,56, 1’234.56) and the week's first day — used wherever New Dawn shows a date, a time or a number (the CRM's tables, boards, calendar and charts, meetings, Tasks, Today, settings); a change redraws every window at once.
- Migration **0031** (`crm_proposal`), reviewed by migration-safety (SAFE WITH CHANGES, all taken: the kind and status checked in Rust like the other enumerations; the one-source check kept with both keys cascading; the record a proposal belongs to re-checked at Apply and moved when people are merged; plan §6 renumbered — Track B now 0032–0043, 0.11.3 and 0.11.4 apart). No new dependency.
- Seen in the dev app (light; Waiting for you and Today in dark too): the CRM's new tab empty — its two ghost cards; the meeting "Building defects review" (88 min) read by Find CRM proposals on qwen3.5:9b in 90 s — 18 proposals: the companies SQC, An Phong and "Mattel Rai"; the people Giang, Hưng and Minh (Mr. Tom and Ms. Chi, already in the CRM, weren't proposed); two opportunities on Marina Center in the Meeting stage — "Core Drilling Verification", quoting "Giang presented the proposal from SQC regarding core drilling to verify concrete quality and validate ultrasonic testing results", and "Defect Remediation (Cracks)"; three notes and seven tasks on them, each quoting the summary. "Mattel Rai" was a person ("Mr. Mattel Rai" in the summary): a name the text writes after Mr., Ms., anh or chị is now never a company, and the guide says so. Marina Center's page showed the two opportunities waiting about it, above its tabs. **Apply** on "Core Drilling Verification" (the card's own button) made the opportunity — Meeting, Marina Center, by AGENT · CRM agent — and the tab read 17; with a close date 12 days on, Today listed it under Opportunities to close ("Closes 14 Nov · Marina Center · Meeting"). On the opportunities table, the switch to year-first dates and dots-and-comma numbers (from the code, without a reload) turned "14 Nov 2026" into "2026-11-14" and "120,000 USD" into "120.000 USD" at once. Settings → General → Formats showed each choice on today's date (2026-11-02, 12-hour (12:57 PM), 1.234.567,89, Sunday). Waiting for you then listed its sources' proposals in reading order (companies first) — they had come tasks first — and stood off the tabs. Everything made was deleted afterwards and the formats set back.
- Review: the architecture review found four must-fixes, all taken — a company's, a person's and an opportunity's page always show their Waiting for you section, ghost cards when none (only those objects have one); no duplicate records: names match whole words ("Lan" isn't in "plan", "Tin" isn't in "meeting", so a CRM full of such names no longer crowds out the people really named), and Apply on a company, person or opportunity already in the CRM by that name is that record; Tasks' "Next week" is next week's Friday whichever day starts the week; Today's Opportunities to close lists the ones to come first and past ones only within 90 days, and "won" is the stages' last option (renamable). Taken too: a change of formats draws the page again, not the whole window (the update check, the code steps' runner, the sidebar and open dialogs stay; the Settings window shows each choice in its own format); the calendar and the meetings' groups place a time on its day in the chosen zone, as do Today's meetings; a zone the system doesn't know is read as this Mac's, so it can't break a window; a company or record waiting from another meeting or email is applied with the proposal that needs it, and a dismissed company leaves a person or an opportunity without one; a task applied takes this meeting's own task first, and one made is the owner's, with its meeting; an amount written "45.000" is left out and huge ones can't overflow; the person titles checked are the English ones (anh, chị, ông and bà are everyday words too); a meeting's Find says how many of its proposals wait; the tab says "Reading…" while it loads and "200+" past 200; a re-read summary keeps the proposals that quote its title. Left: a time typed in a cell is this Mac's (the hint says so); the agent may still call a piece of work an opportunity — Dismiss is the answer.
- Checks: 196 Rust tests pass (four new: what an answer keeps — quotes in the text, names not in the CRM, never the owner nor a person called a company, stages, amounts, dates, companies and records named beside; applying once with what it depends on — the company first, from another source too, a task attached rather than doubled, a dismissed company, a record already there, the ledger; names matching whole words among seventy look-alikes; the formats kept and read back — and Today's test now covers the opportunities to close and the three week starts); the Tasks and meetings-list scripts pass, with Sunday and Saturday weeks; clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. Formats start as New Dawn's own (24 Oct 2026, 24-hour, 1,234.56, Monday).

---

# Last Build — Developers: API keys, local REST API, webhooks · release 0.10.14
_2026-11-02_

## What changed
- **The local API** (D-071, Twenty's REST API on this Mac): Workspace → CRM → **Developers** → Local API On — New Dawn listens on `http://localhost:<port>` (127.0.0.1 only; the port is picked once and kept; off by default), with **Copy address** and **Open the docs**. Twenty's paths and JSON: `GET /rest/people?limit=2&filter=…&orderBy=…&starting_after=…` → `{ data: { people: [ … ] }, pageInfo, totalCount }`; get, create (POST), update (PATCH), delete (to Deleted records), `POST /rest/batch/<objects>` (60 at most); Twenty's filter syntax (`name[eq]:"Acme"`, `or(…)`, `not(…)`, `name.firstName[ilike]:"%an%"`, `amount.amountMicros[gt]:…`) and orderBy; errors in Twenty's shape; `companyId` for the company. The docs page lists the endpoints, the syntax, every workspace's objects and fields, and curl lines.
- **API keys**: name and expiry (Never, 30 days, 90 days, 1 year) → the key is shown once ("nd_…", Copy) and only its fingerprint is kept; the list shows each key's prefix, when it was made and last used, and Revoke.
- **Webhooks**: an address on this Mac (anything else is refused) and the events it wants (`*.*`, `person.*`, `opportunity.updated`); each record made, changed or deleted is posted as `{ event, data, timestamp }`, signed like Twenty's (`X-Twenty-Webhook-Signature`, HMAC-SHA256 of `timestamp:body` with the webhook's secret, shown under Deliveries); on/off, Delete, and the last 50 deliveries with their status, time and error.
- **Workflows**: a workflow with a **Webhook** trigger now has its address — `POST http://localhost:<port>/webhooks/workflows/<workspace>/<id>` — shown in the builder when the API is on; the JSON body is the run's trigger data.
- Migration **0030** (`api_key`, `webhook`, `webhook_delivery`), reviewed by migration-safety (SAFE; its notes taken: the delivery log pruned with a NULL-safe query, the expiry written like "now"). Dependency added: `axum` 0.8 (the HTTP server plan §5.8 names). Record events now have several listeners (the workflows, the webhooks).
- Seen in the dev app (light; the dark run's window opened where it couldn't be captured): Workspace → CRM → Developers with the Local API row off, its keys and webhooks groups and their ghost rows; Local API On listened on 127.0.0.1. With a key, `curl …/rest/people?limit=2` answered Twenty's JSON — two people, `pageInfo` (hasNextPage, startCursor c0, endCursor c1), `totalCount` 17; `filter=name[ilike]:"%Keppel%"` on companies answered Keppel Land alone; without a key, 401 `{"error":"UnauthorizedException", …}`; `/docs` opened on "New Dawn — local API". A webhook added through the panel's form (an address on this Mac, `person.updated`) got the event when a person's job title changed — its signature matched the secret — and Deliveries showed it, 200. "New key" in the panel showed the key once, with Copy. A workflow with a Webhook trigger, activated, ran on `curl -X POST …/webhooks/workflows/vifc/<id>` with `{"title":"Hello from curl"}` and made the note "Hello from curl" by WORKFLOW. After the review's fixes, again on the dev app: a foreign `Host` or `Origin` got 403 (`ForbiddenException`), a `localhost` Origin 200; a filter nested 40 deep got 400 "The filter nests more than 16 deep"; `PATCH {"name":{"firstName":"Patched"}}` kept the last name. Everything made was deleted afterwards and the API turned off.
- Review: the architecture review found three must-fixes, all taken — odd filter input can't crash or hang the app (a non-breaking space between conditions, a list that isn't closed, groups or lists nested past 16 are refused); "this Mac only" holds against a browser too: a request's Host must name this Mac and a browser's Origin, when sent, too, so a web page reached through DNS rebinding, or a form on one, gets 403 (the docs and a workflow's webhook take no key); a PATCH with part of a name, an email or an amount keeps the other parts. Taken too: a batch checks every value before writing any; `ending_before` gives the page before the cursor, never the cursor's record; the webhook trigger's lookup lives with the workflows' triggers; a workflow's HTTP step can't call New Dawn's own API (a workflow writes through its record steps, whose writes start no trigger); a key's last use is written at most once a minute; one client for every delivery; turning the API on and off happen one at a time, and off waits for the port to close; the panel says "On, but not listening" when it couldn't start; the docs say `startsWith`, `like` and `ilike` match anywhere; the panels start afresh on another workspace. Left: one record's events can reach a webhook out of order (each event's deliveries are their own task); a batch that fails in the database part-way keeps the records before it.
- Checks: 192 Rust tests pass (five new: Twenty's filters and orders read — and odd input refused; the REST answers on real records — pages, cursors both ways, a filter, create, get, PATCH keeping a name's other part, batch with one bad value writing nothing, delete, errors in Twenty's shape, keys expiring and revoked, Host and Origin read; HMAC-SHA256 against RFC 4231; a delivery signed and logged; events matched by object and action); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. The local API stays off until you turn it on in Workspace → CRM → Developers.

---

# Last Build — AI chatbot and agents · release 0.10.13
_2026-11-02_

## What changed
- **Ask AI** (D-070, Twenty's AI chatbot on the local model): ⌘K → "Ask AI" — or, on a record, ⌘K → "Ask AI about this company" and the record's ⋯ menu — opens a glass panel on the right. Ask in plain words: the assistant looks things up in the CRM and answers with links to the records it found ("which opportunities above 50k have no activity for 30 days?"), each lookup shown in a quiet line ("Looked up opportunity records — 1 found"), the model named under each answer, the AI progress bar while it works. Follow-ups keep the conversation; "this company" means the record the chat is on. The chats of a place are kept (newest first, New chat, Delete this chat).
- **Changes are proposals**: "add a note to Keppel Land: …", "move Fit-out to Customer", "create a company …" come back as a card — "Add a note to Keppel Land — “…”" — with **Apply** and **Dismiss**; nothing is written until you apply, and then it's written once, as the agent (the record's history says so). A note is attached to its record.
- **AI agents** under Workspace → CRM: the default "Assistant" and your own — a name, a description, instructions, a model (the app's or any installed one) and the tools it may use (find, count, sum/average, open; propose a record, a change, a note). A workflow's **AI agent** step can now pick an agent: its instructions, model and lookups (never a proposal inside a run).
- Migration **0029** (`crm_agent`, `crm_chat_thread`, `crm_chat_message`), reviewed by migration-safety (SAFE; its notes taken: the log row's kind, one default agent even when asked twice at once, threads on a record deleted for good kept and detached, the thread's time follows its last message, a proposal applied or dismissed once). The AI gateway gained a chat turn with tools (Ollama's `/api/chat` `tools`), logged like every completion. No new dependency.
- Seen in the dev app, driven through the panel's own controls (the window opened where it couldn't be captured, so there are no screenshots this time; what follows is what the database recorded): three opportunities on Keppel Land, two quiet since 1 September; ⌘K → "Ask AI"; "Which opportunities above 50k have no activity for 30 days?" — the model's first lookup carried an empty filter, the tool told it so, it looked again and answered "There is 1 opportunity above 50k with no activity for 30 days: Tower B lease (120,000 USD, stage Proposal, company Keppel Land)" with the record's link; "Add a note to Keppel Land: call back about the Tower B lease next week." — it proposed "Add a note to Keppel Land — “Call back about Tower B lease”", naming the company rather than giving its id, and said it was proposed; Apply on the card wrote the note, by AGENT · Assistant, attached to Keppel Land. The model then linked Keppel Land with a link of its own making; a proposal's result now carries the record's link. The record's "Ask AI about this company" and Workspace → CRM → AI agents weren't seen this time. Everything made was deleted afterwards.
- Review: the architecture review found one must-fix, taken — applying a proposed amount would have erased the record's currency (only the amount was proposed); a proposed value is now written whole (an amount with the record's or the field's currency, a name split in two, an email, a phone or a link as its first part, rich text as the editor's document). Taken too: an update never lands on a record deleted since; a note can't be proposed where it can't be attached, and is undone if its attachment fails; objects turned off are out of reach, as are workflows, runs and dashboards (no "Ask AI about this" on them); numbers given as text count (`"30"` days); lookups take an order and say when matches went past the 2 000 they look through; sums and averages refuse a field that isn't a number or an amount; earlier turns' results are read back in brief so long chats don't overflow; a question isn't kept twice when the model can't be reached; a record may be named instead of given by id; the summary shows labels; the Open link after Apply uses the object's plural; a chat is deleted only after a confirm; the examples are ones the tools can answer. Left: during a Live session the panel says "Thinking" while it waits for Live to stop (nothing can go ahead of Live); the thread list reads each chat's record label separately.
- Checks: 187 Rust tests pass (two new: the tools on real records — the quiet deals over 50k with their links and last activity, a count by a select's label and a relation's name, a sum, a mistake told back, a tool the agent may not use refused; a note proposed by the company's name in a scripted turn, nothing written before Apply, written once after it with its target and the agent's name; an amount changed by name keeping its currency; a dismissed change writing nothing; a chat kept and detached when its record is deleted for good — and the agents: one default even when asked twice at once, tools limited to the known ones, the default kept, a deleted agent's chat going on with the default); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. The assistant uses the model selected in Settings → Models unless an agent names another.

---

# Last Build — Workflows II · release 0.10.12
_2026-11-02_

## What changed
- **Four more steps** in the workflow builder (D-069, Twenty's AI Agent, Send Email, HTTP Request and Code, adapted to this Mac): **AI agent** — a prompt with variables on the local model (the app's, or one picked from Ollama's installed models) through New Dawn's AI queue; **Outputs** (a name and Text, Number or True / false) make the model answer in JSON and each output a variable (`{{s4.summary}}`), else the answer is `{{s4.text}}`. **Send email** — To (one address or several), Cc, Subject and Body with variables make a draft: the run's page shows it with **Open in Mail**, and you press Send in Mail (New Dawn never sends, nor opens Mail by itself). **HTTP request** — GET, POST, PUT, PATCH or DELETE to an address on this Mac (localhost) with headers and a body; any other address is refused with "Only this Mac's addresses (localhost) can be called: … is on the network" — at Activate when it's written out, at the step when it comes from a variable; 30 s, 10 MB, no redirects; `{{s2.status}}` and `{{s2.body}}` after it; an answer that isn't 2xx stops the run. **Code** — JavaScript, `export const main = async (params) => { … return { … } }`, its inputs with variables, its result the output; **Test** runs it with the inputs as typed; it runs in New Dawn's window in a sandboxed worker (no network, 10 s at most) — with the window closed the run waits, and its page says "Waiting for New Dawn's window to run the code".
- The form's button reads **Submit** (Twenty's word). The content policy allows the code step's worker (`worker-src blob:`); nothing else changed in it.
- No migration, no new dependency.
- Seen in the dev app (light; the run page and the email step in dark): a workflow "All four" (by hand) with an HTTP request "Ask the local service" (GET to a server on 127.0.0.1 answering `{"greeting": "hi there"}`), a Code step "Shout it" (input `word` = `{{call.body.greeting}}`, returning `{ shout }`), an AI agent step "A line from the model" (qwen3.5:4b, "Write one short, friendly sentence that says {{calc.shout}} to Minh.") and a Send email step "Draft the note" (To minh@example.com, Subject `{{calc.shout}}`, Body `{{ask.text}}`); the inspector showed each step's settings with labels above the controls, the `{x}` picker beside each text field, the code in the mono face with Test, the model picker on qwen3.5:4b. ⌘K → "Run All four" started it: the run waited for the window at the code step, the window ran it within seconds ("HI THERE"), the model answered "Hi there, Minh! 👋", and the run Completed in 0:07 — the four steps ✓, and under the email step "A draft to minh@example.com — HI THERE" with Open in Mail (not clicked: the check never asks Mail); the local server logged the GET. A second workflow "Away" with an HTTP step to example.com stayed a draft: Activate showed "Only this Mac's addresses (localhost) can be called: example.com is on the network". Everything made was deleted afterwards.
- Review: the architecture review found three must-fixes, all taken — the HTTP step's client ignores proxies (a system or environment proxy would have carried a "localhost" call, with its body, off the Mac); a code step's answer counts once (each wait has an attempt token, and an answer — a form's too — goes into the run only once the run is claimed, so a late answer never lands on an item that moved on); and slow steps no longer hold the others (each record event's runs and each workspace's minute run on their own task; one workflow AI step in the AI queue at a time; a schedule's slot taken atomically). Taken too: a body written as JSON resolves as JSON (values escaped, a lone `{{x}}` keeps its type) and a record or list in an AI prompt goes in as JSON; Ollama's port is refused by the HTTP step (the model only through the AI step and its queue); nested workers are cut off and only main's own answer counts; code steps of workflows in Deleted records don't run; the AI answer's JSON is found past stray braces, and outputs can't be called text or model; a running run can be cancelled (it stops before its next step); the run refreshes the same lists everywhere; a header that doesn't read says so; 1 MB of an HTTP answer is kept in the run; a code step that took the window down isn't run again on reload; Mail's refusal to open a draft shows. Left: the AI activity list doesn't name the workspace of a workflow's AI step.
- Checks: 185 Rust tests pass (one new: a local server answers and a 404 stops the run; addresses on the network and Ollama's port are refused; a stray brace doesn't hide the AI's JSON; the draft link is encoded and a To without an address stops the step; the code step waits for the window, a wrong or late attempt is refused, an error fails the run and Retry asks again; a written-out network address is refused at Activate); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Workflows I · release 0.10.11
_2026-11-02_

## What changed
- **Workflows** (D-068, Twenty's): a new object in the CRM — Workflows and Workflow runs in the sidebar's Workspace section and the CRM's tabs, "Create a workflow…" in ⌘K, ★ to pin one. A workflow's page is **the builder**: the flow on the left — the trigger card first, then the steps as cards, an If/else's routes and Else and an Iterator's inner steps indented, ▲ ▼ to reorder, ✕ to remove, "+ Add a step" at the end of every list with the picker of step types — and the chosen card's settings on the right. **Triggers**: a record changes (an object; is created / updated / created or updated / deleted; the fields watched), by hand (everywhere from ⌘K; on a record, once a record; on records, once with them all), on a schedule (every N minutes / hours / days, or a cron in this Mac's time), a webhook (shown; its URL comes with the local API). **Steps**: Create record (an object, its writable fields each with the control the field wants — a select, On/Off, a part each for a name or an address, text with variables for the rest —, "Attached to" for a task or note), Update record (the record from the trigger or a step), Delete record, Search records (the view bar's rules, at most 200), Upsert record (match by id, email, domain or a unique field), Filter (conditions; the run goes on only when they hold), If/else (routes with conditions, Else), Iterator (the inner steps once an item), Delay (a wait or a date), Form (title, instructions, fields: text, number, date, select — the run waits for you). Every text setting has a **variables picker** `{x}`: the trigger's record and its fields, the record steps before it, a search's records, a form's answers, the route taken, the iterator's current item and index, dates (today, now, tomorrow, in 3 days, in a week, a week ago); typed by hand they read `{{trigger.record.pointOfContact.id}}`, `{{s2.name}}`, `{{today+3d}}`.
- **Versions**: the builder edits the draft (made from the live version the first time); **Save draft**; **Activate** publishes it — the live version goes to the archive, the triggers are set, the workflow reads Active; ⋯ → **Deactivate**, **Discard draft**; the Versions list on the right (Draft, Active, Archived, Deactivated, with their dates) with **Use as draft** on a past one. **Test** runs the draft — with a record searched and picked when the trigger wants one — and opens the run.
- **Runs**: a Workflow run is a record ("<workflow> · <date>", status, workflow, started, ended); a run's page shows the facts, the error, Retry from the failed step, Cancel run, the **form** the run waits for (its fields; Continue), and the steps with their state (done ✓, waiting …, failed ✕, still to come) and each one's output; the trigger's data under them. The workflow's page lists its last runs. A run keeps its place in the database: a delay or a form leaves it waiting, and it goes on after a quit — the scheduler (one task a minute over the open workspaces) resumes the delays over and starts the schedules due (one run however many ticks were missed).
- **Running by hand**: ⌘K lists "Run <workflow>" for the global manual workflows and "Run <workflow> on this <object>" on a record's page; a record page's ⋯ lists "Run <workflow>" for the manual workflows that take a record of its object. A run starts in place and its page opens.
- **The engine tells, it never calls**: `kernel/records/events.rs` is one bounded channel of record events (created, updated with before and after, deleted; the actor's source) emitted after the timeline row; the workflows module listens; a workflow's own writes start nothing. Record steps write through the engine with the actor "WORKFLOW · <workflow>", so the timeline and the search follow.
- Migration **0028** (`workflow`, `workflow_version`, `workflow_run`, `workflow_automated_trigger`), reviewed by migration-safety (SAFE WITH CHANGES, all taken before shipping: one Active and one Draft a workflow as partial unique indexes, an index on the triggers' workflow; and in the code, a workflow in Deleted records answers no trigger, and one workflow's trouble never stops the others' runs or the delays). No new dependency.
- Seen in the dev app (light; the builder and a run in dark): a new workflow "Welcome customers" opened on the builder — the trigger card ("Choose a trigger"), a ghost step and the settings aside with Versions and Runs; the trigger set to "A record changes" on Opportunities, "is updated", Stage watched (the card read "opportunity is updated · stage"); "+ Add a step" opened the picker of ten step types; a Filter "Only customers" with `{{trigger.record.stage}}` is `CUSTOMER`; a Create record "Welcome task" on Tasks with Title "Send welcome note to {{trigger.record.pointOfContact.label}}", Due date `{{today+3d}}`, Status To do and "Attached to" People `{{trigger.record.pointOfContact.id}}` — each field's label above its control, the draft saving as it was typed; Activate made v1 Active with one record trigger and no extra draft (the bar then read Active, Activate off). An opportunity "Bank portal" moved from Proposal to Customer through the engine made the task "Send welcome note to Duc", due 5 Nov 2026, by "WORKFLOW · Welcome customers", attached to Duc; its run page read Completed, "A record changes · opportunity is updated · stage", the two steps ✓ Done with their Output, the trigger data under them. A second workflow "Say hello" (by hand, everywhere: a Form "Which greeting?" then a note) showed in ⌘K as "Run Say hello" under Workflows; running it opened its run Waiting with "The run waits for you", the Greeting select and Continue, the steps "? Waiting for you" and "· still to come"; Hello + Continue completed it (Duration 0:06) and made the note "Hello from a workflow". Everything made was deleted afterwards (workflows, versions, runs, triggers, the task, the note, the opportunity). The `{x}` picker's fixed width, the last CSS change, was checked in code only: by then the dev window opened where it couldn't be captured.
- Review: the architecture review found six must-fixes, all taken — what the owner types can't crash the app (a date or a wait off the calendar, a cut character after `{{today+3`, a schedule that never comes are empty or refused); an iterator's item starts with the item before's outputs cleared (a delay waits and a form asks again for each item); an If/else keeps its route when a run resumes; "Use as draft" reaches the builder; a step that would make a record with an empty title or name stops with "… came out empty"; a step's output JSON uses the mono token. Taken too: a run claims its status (a cancelled or finished run never comes back; Cancel only while it waits; a form answered only while the run waits on it); runs a quit stopped mid-step are marked failed at the next launch, Retry going on from their step; a run on records names their object; workflows and their runs are no trigger's or step's object; a relation in `{{…}}` gives its id to a record step; reading a workflow never makes a version (the first edit does) and Discard is off for the only version; the builder saves as you go and when the page closes, keyed on the workflow; a record a step made isn't made twice on Retry; a test run whose draft was discarded fails plainly; a workflow's own writes and imports aren't announced; runs aren't in search; and the nits (a route without conditions never holds, numbers with commas, composite values in sentences, nested iterators, cards that keep focus, variables only from steps that ran plus the record before an update and a schedule's time, ⌘K shows a refused run). Left: the definition's TypeScript types are kept by hand beside the Rust ones; the record-event test shares the process's channel.
- Checks: 184 Rust tests pass (six new for workflows: a stage change makes the task on the point of contact through the record event and nothing else starts one; a delayed run waits and goes on at its time, through If/else, an Iterator and a Filter that ends it; each iterator item asks its form again, a form is answered only while waited on, and a finished run is neither retried nor cancelled; extreme dates, waits and schedules fail plainly; templates and conditions read the context; cron and intervals find the next minute; the navigation test now lists the new objects); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Dashboards · release 0.10.10
_2026-11-02_

## What changed
- **Dashboards** (D-067, Twenty's): a new object in the CRM — Dashboards in the sidebar's Workspace section and the CRM's tabs, "Create a dashboard…" in ⌘K, ★ to pin one. A dashboard's page is its own layout: tabs of widgets on the 12-column grid, in view mode; **⋯ → Edit dashboard** opens the layout editor (0.10.4's: + Widget, drag by the handle, resize by the corner, tabs added, renamed, reordered, removed; Save, Cancel) with the dashboard palette — **Chart**, **Table**, **Rich text**, **iFrame**; **Duplicate dashboard** (⋯ and ⌘K) copies the record and every tab and widget.
- **Charts**: a widget's settings are Twenty's — Chart (Bar · Line · Pie · Aggregate), Source (an object), Filter (+ Add filter, the view bar's rules), X axis (a select, a date with its granularity — Day, Week, Month, Quarter, Year, Day of week, Month of year, Quarter of year —, a relation, a boolean, text), Y axis (Count all, Count empty, Count not empty, Count unique values, Sum, Average, Min, Max, Percent empty, Percent not empty, Count true, Count false, Ratio of a value; the field), Group by (a second field makes series, its own granularity), Sort by (Position, Alphabetical, By value), Omit zero values, Cumulative; Horizontal bars, Stacked, Data labels, Legend, Axis name; a pie's "Each slice represents", Show value in centre, Hide empty category; an aggregate's Prefix and Suffix. The charts are inline SVG in the Majoris families (a select's option colours map to them), with the total in a pie's centre and a note when more than 100 categories or 50 series were left out. A chart refreshes with the records: a card moved on the kanban moves the bar.
- **Table**: an object's view on the dashboard — its columns, filters and sorts — read-only unless "Allow editing". **iFrame**: a `localhost` address in a frame (nothing on the web). **Chart on a record page**: in the record page's layout editor, a Chart of related records (the one-to-many relation to chart, filtered to the record — a company's opportunities by stage).
- Migration **0027** (`dashboard`, `page_layout.dashboard_id`, `favorite.dashboard_id`), reviewed by migration-safety (SAFE; its one change taken before shipping: one layout a dashboard — a partial unique index — and the reader makes it with insert-or-ignore, so two asks at once make one). No new dependency; the chart tokens `--chart-1…8` and `--chart-grid` join `tokens.css`; the CSP's `frame-src` allows localhost.
- Seen in the dev app (light; the dashboard in dark): three opportunities made through the engine on Keppel Land; ⌘K → "Create a dashboard…" → "Pipeline" opened the dashboard's page (empty, with its ghost and the hint); ⋯ → Edit dashboard, + Widget → Chart: the settings read Title, Chart Bar, Source Opportunities, Filter (+ Add filter), X axis Stage, Y axis Sum, Field Amount, Group by, Sort by Position, Omit zero values, Cumulative, Horizontal bars…; a second Chart as a line (Close date by Month), an aggregate of people, a Table of the first companies view and a Rich text; Save kept the five widgets (the charts six columns wide, the table and the text twelve); the page then showed "Pipeline amount by stage" as bars in the stages' own colours with a `$` prefix — every stage, Customer at zero — and "Opportunities closed by month" as a line; a stage changed through the engine showed on the next open ($120k moved from New to Screening); ⌘K on the dashboard offered "Duplicate dashboard" and made "Pipeline copy" with the same widgets; the dashboards appeared in the sidebar's Workspace section as "D Dashboards". On Keppel Land's page, Edit page layout → + Widget → Chart with "Opportunities" as the related records, a pie of Sum of Amount by Stage, saved: the ring showed New 33 %, Screening 12 %, Meeting 55 %, Proposal and Customer 0 % in the stages' colours with $365k in its centre. (A change made outside the app's own mutations shows on the next open; one made through the app refreshes in place, as the chart's query shares the records' key.)
- Review: the architecture review found one must-fix, taken: Earliest / Latest on a date field would have charted 0 (a value is a number) — they are out of the list until a chart can show a date. Taken too: a date-time is bucketed in this Mac's own time; a relation's labels are read for the kept keys only, after the grouping; the categories and series are gathered in sets with their totals computed once; a ratio on a multi-select reads the JSON list with the value bound; the iFrame takes `http://localhost` only (the policy allows no `https` there); React keys are the categories' keys; the chart labels use the type scale and the legend's swatch is a dot; the pie's row is a class; Duplicate dashboard calls the mutation directly from the menu; the loading dashboard says "Reading…". Noted and left: a duplicate's record and layout are two transactions; `%W` counts weeks from the first Monday; numbers on the X axis sort as text.
- Checks: 178 Rust tests pass (two new: a sum of amounts by stage keeps every option in order with zeros, a count by close month with a filter and cumulative, a count by relation with series by stage sorted by value, a ratio; a dashboard's layout is made once, refuses a record widget, and a duplicate carries the tabs and widgets to the copy); clippy, tsc and the localhost check are clean.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Emails and calendar from local sources · release 0.10.9
_2026-11-02_

## What changed
- **Emails on a record** (D-066, Twenty's Emails tab on people, companies and opportunities): the threads from Apple Mail where any of the record's addresses took part — a person's emails; a company's people's emails and anyone at its domain; an opportunity's company's — newest first: the subject, the people (a name links to its person, or to the company owning the domain), when and how many messages, the first words (per the visibility setting); a thread opens to its messages, each with **Open in Mail**; **Refresh** asks Mail again (it is asked when the tab opens and the last read is older than ten minutes); **Send email** opens a draft in Mail. Mail must be running — New Dawn never launches it — and allowed under Automation on the first read; the widget says so when it isn't. Under the threads, **Not in the CRM** lists the people a thread names that the CRM doesn't know (per Contact auto-creation; never your own address, never a blocked one) with **Add person** (the record, its company by the domain) and **Block**.
- **Calendar on a record**: the summarised meetings whose attendees include the person — a company's people, an opportunity's company's — newest first: the meeting (a link), when and how long, a **Recording** chip when the audio is kept, the attendees linked to their records. The record pages of people, companies and opportunities get **Emails** and **Calendar** tabs in their default layout (a saved layout keeps its own tabs; the widgets are in the layout editor's list).
- **Settings → Accounts** (the Settings window, group App): Apple Mail as the connected account — whether Mail is running, what is read and the Automation note; **Contact auto-creation** (Deactivated · Sent & Received · Sent only); **Email visibility** (Metadata only · Subject + metadata · Share everything); the **Blocklist** — addresses or @domains with Remove, and a field to add one (blocking empties the threads kept, so what they sent is gone at the next read). The settings are the current workspace's.
- Migration **0026** (`message_thread`, `message`, `message_participant`, `blocklist`), reviewed by migration-safety (SAFE; its nits taken: one composite index on the messages, `bcc` allowed in the roles, a block empties the cache, the setting's timestamp). The opener's URL scope allows `message:` and `mailto:`.
- Seen in the dev app (light; the Emails tab in dark) with Mail rows seeded in the database (the check never asks Mail — Mail's content stays the owner's): Mr. Tom's page grew **Emails** and **Calendar** tabs; Emails listed three threads newest first — "Lease renewal for tower B · 30 Oct 2026 at 15:40 · 2 messages" with Lan Pham (linked to Marina Center by her domain), Tien Nguyen, Tom Nguyen (linked to Tom) and the latest words, "Weekly digest", "Parking allocation" — each with Open in Mail, the head "Read from Mail 2 Nov 2026 at 03:46 · Refresh · Send email"; the thread opened to its two messages ("Tien Nguyen · 30 Oct 2026 at 15:40 · Sent — Thank you Tom…", "Tom Nguyen · 28 Oct 2026 at 09:12 · Received — Dear Tien…"); **Not in the CRM** listed Lan Pham and the digest's sender with Add person and Block, and Add person made Lan Pham a person on Marina Center (the row then gone, her name a link); Marina Center's **Calendar** listed "Building defects review · 29 Oct 2026 at 09:38 · 88 min · Recording" with its attendees, the known ones linked; Settings → **Accounts** showed Apple Mail ("Mail is not running"), Contact auto-creation (Sent & Received), Email visibility (Subject + metadata) and the Blocklist. The AppleScript compiled with `osacompile`.
- Review: the architecture review found two must-fixes, both in the Mail reader, and both taken: under "Metadata only" every message had keyed to one thread ("(no subject)") shared by every record — a thread's key is now the subject with the other side's domains, and a message without a subject is a thread of its own; and the script had asked Mail one event a message (seven of them, plus one a recipient), which a large mailbox turns into minutes and a killed read — it now reads each property of a whole list in one event, bounded by a year back, the newest hundred an address kept in Rust, and an address at a domain also asked for is left out; the inbox rows carry no recipients (the sent rows do). Taken too: the owner's own addresses come from Mail's accounts (they were only known once a sent message had been read, so your own address was proposed as a person); what Mail answered is kept in one transaction with the thread's roll-up once; a visibility change makes the next open read again; the Blocklist field adds on Enter or Add, never on leaving it (a block empties the cache); the Calendar is one list, newest first (a summarised meeting is past by nature); one script runs against Mail at a time; Refresh shows busy during the automatic read too; the settings' placement is in D-066. Noted and left: `opener:default` already allows http, https, mailto and tel, so the new scope adds `message:` without narrowing (pre-existing); a block empties the whole cache rather than that sender's messages.
- Checks: 176 Rust tests pass (two new: subjects, senders, blocks and Mail's rows read right; what Mail answered is kept as threads and participants, a person's threads come back newest first with the people linked and an unknown address proposed per the setting or not when blocked, adding them makes a person on the company of their domain, a company sees its people's and its domain's threads); the AppleScript compiles (`osacompile`); clippy, tsc and the localhost check are clean.

## Manual steps
- The first Emails tab you open asks for permission to control Mail (System Settings → Privacy & Security → Automation); allow it once. Mail must be open for a read.

## Env / config
- Capability: `opener:allow-open-url` with `message:*` and `mailto:*`. No new dependency.

---

# Last Build — Import and export · release 0.10.8
_2026-11-02_

## What changed
- **Import records** (D-065, Twenty's spreadsheet import), from a view's ⋯ menu or ⌘K ("Import people from a file…"): a sheet in three steps. **Select file** — a CSV, XLSX or XLS (up to 10 000 rows; the first sheet; dates and numbers read as text). **Match columns** — every column against the object's fields, matched by themselves from Twenty's "Phones / Primary Phone Number" headers, the fields' labels and names, a part's name on its own (firstName, email, website…) and the usual relation columns (companyDomain, companyId, company); a composite's part or its whole (a "Name" column splits into first and last); a relation by the other object's id, label, unique field, domain or email (**Match by**); **Update existing … by** picks the identifier — the id, a person's email, a company's domain, a unique field — so rows that match update instead of duplicating. **Validate data** — each row New, Update or Error: a cell in error is outlined with the reason under it (not an email, not one of the options, "Can't connect to company: no record whose name is …", "Name is required", "the same value is on another row", "already used by another person"); cells are edited in place and checked again as you type, × removes a row, "Show only rows with errors"; Confirm imports the clean rows and skips the rest ("Import 2, skip 1"). Then the report: created, updated, skipped, the errors by row. A blank cell clears the field on an update and leaves the default on a new record; the actor on the records and the timeline is the import, named after the file.
- **Export view** from the ⋯ menu or ⌘K: the save dialog, then a CSV with exactly the view's visible fields in order (Id first; a composite as one column a part — "Name / First Name"; a relation as its label and "Company / Id"; TRUE / FALSE; RATING_n; a rich text as its words), with the view's filters, sorts and the search, up to 20 000 rows; then "Exported N records" and Reveal in Finder.
- **Imports and exports** at the bottom of Workspace → CRM → Data model: the last fifty runs with their counts and the exported file (Reveal).
- Migration **0025** (`import_job`, `export_job`), reviewed by migration-safety (SAFE; its code nits taken: UNION ALL, v7 ids, the first hundred errors kept). Dependency added: `calamine` (XLSX / XLS / ODS reading, MIT). `kernel/csv.rs` reads and writes CSV with the standard library.
- Checks: 174 Rust tests pass (three new: the CSV round trip with quotes, commas and line breaks; Excel serial dates, numbers, hosts and ids read as text; a Twenty-style People CSV maps by itself, imports with the companies linked, a second import with new titles updates instead of duplicating, a domain nobody has is an error on its row, and the export of the people view has exactly its columns with the Id first); clippy, tsc and the localhost check are clean.
- Seen in the dev app (light; the sheet's Validate data step in dark): on People, ⋯ → Import records opened the sheet; a Twenty-style file (firstName, lastName, email, jobTitle, company; three rows, one naming a company that doesn't exist) mapped by itself — Name / First Name, Name / Last Name, Emails / Primary Email, Job title, Company matched by Name — with "Update existing people by Emails"; Next step showed 2 to create · 0 to update · 1 with errors, the third row yellow with "Can't connect to company: no record whose name is Nowhere Corp" under its company cell; "Phu Long" typed into that cell made the row New and the button "Confirm"; Import done read 3 created, and the three people were in the table with their companies and the import as their actor (IMPORT · people-import.csv); ⋯ → Export view, and ⌘K → Export view, wrote the CSV (Id, Name / First Name, Name / Last Name, Emails / Primary Email, Emails / Additional Emails, Linkedin / …, X / …, Job title, Phones / …, City, Avatar, Company, Company / Id; 20 rows) and showed "Exported 20 records" with Reveal in Finder; Workspace → CRM listed "Export · All People — 20 records" and "Import · people-import.csv — 3 created · 0 updated · 0 skipped of 3"; ⌘K listed "Import people from a file…" and the other objects'; in dark, the same file again showed its rows as Update (the emails matched) and the error row.
- Review: the architecture review found no must-fix; taken: the ⋯ menu's Export view goes through the same URL flag as ⌘K (it had passed an empty search and kept its own copy of the status); reading a file and writing the export run off the async workers; the sheet's heading is the shared one, its cells 28 pt, the button reads "Next step"; an import refreshes the records, the open record and the jobs rather than every query; the jobs list's empty state is two ghost rows; a Reveal that fails (the file moved) shows its reason. The review's trust note is in D-065 (the paths come from the native dialogs, unscoped, for a local single-user app whose webview runs bundled code only; nothing from a file reaches the SQL text). Noted and left: ⌘K's import entries land on the object's default view.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

## Env / config
- Dependency added: `calamine` 0.35 (Rust). No new capabilities (the export writes where the save dialog points, from Rust; Reveal uses the opener plugin's default permission).

---

# Last Build — Sidebar favorites, the Workspace section, ⌘K, shortcuts · release 0.10.7
_2026-11-02_

## What changed
- **Favorites in the sidebar** (D-064, Twenty's Favorites): ☆ on a record's page pins it (★ once pinned); a view's ⋮ in the view dropdown has **Add to favorites**; the sidebar's CRM section lists the pins — a record by its label, a view by its name — in folders. ✎ opens the customise mode: ▲ ▼ to reorder, × unpins, a folder per pin, **New folder**, delete a folder (its pins come back to the root); Save or Cancel. A pin whose record is in Deleted records is left out, and back after a restore.
- **The Workspace section**: every record object (a custom one joins at the end) in the order the owner set, hidden ones left out (the eye hides or shows one in the customise mode), in named folders. Nothing of this shows in the icon rail.
- **⌘K — one command menu for the whole app**: **Recent** (the records opened lately — a record's page notes each visit; the last hundred are kept), **Records** that match what's typed, **Create a …** for every record object (then the name, then ↩ — the record opens on its page), **Go to** (Today, Meetings, Live, Tasks, Projects, Email, Workspace, Settings; every object; every view by name), and the page's **Actions** (Edit record page layout on a record, Deleted records on an object's tab, collapse or expand the sidebar, Copy link to page, Keyboard shortcuts). ↑ ↓ move, ↩ runs, ⌘↩ runs the first result, Esc closes (or steps back from the name or the shortcuts).
- **Keys**: ⌘K; `/` opens the menu in the records; `g` then `p c o n t` goes to People, Companies, Opportunities, Notes, Tasks, `g m` to Meetings, `g d` to Today (a one-second chord; never while typing or while a sheet is open). The toolbar's field is the meetings search — ⌘K no longer focuses it. The list is the menu's Keyboard shortcuts.
- Migration **0024** (additive: `favorite_folder`, `favorite.folder_id`, `navigation_item`, `recent_visit`), reviewed by migration-safety (SAFE); its nits taken: a record deleted for good, or an object dropped, forgets its visits; the trim uses the index; the index on `favorite.folder_id` is named for it.
- Checks: the record engine's 10 tests pass, the new one included (a record and a view pinned once each, moved into a folder, the folder deleted — the pins back at the root; a deleted record's pin left out; the Workspace section lists every object, reordered, hidden and foldered; one visit per record); clippy, tsc and the localhost check are clean.
- Seen in the dev app (light; the company's page, the sidebar and ⌘K in dark): on Companies, ⌘K then "co" put **Create a company…** first (then the two tasks about core drilling under Records, Go to Companies, the All Companies view, the page's Actions); ↩, "Galaxy Pay", ↩ made the company and opened its page, whose ☆ pinned it; in the view dropdown, ⋮ on All Companies offered **Add to favorites**, and the view then sat in a "Pipeline" folder in the sidebar under the company; ✎ opened the customise mode — the eye hid Notes and "Sales" typed as the folder of Opportunities, Save — and the Workspace section read People, Companies, Tasks, then Sales with Opportunities a step in; `g` then `p` went to People; "keyboard" ↩ showed the shortcuts sheet; in dark, the Galaxy Pay page showed ★ filled, the Favorites with the folder, and ⌘K opened on Recent (Mr. Tom, the task). The first pass of this check found the menu ranking records above "Create a company…" when "co" was typed (now Create ranks first when the text starts an object's name) and the folder rows too faint (now rows like the items); deleting the company for good left no visit behind.
- Review: the architecture review found one must-fix, in: the ★ button was tinted orange (a secondary button; the glyph and `aria-pressed` carry the state). Taken too: the menu's entries are built on every render rather than memoised (a memo held a stale "collapse the sidebar" closure); ⌘K does nothing while a sheet is open (no second modal over Start recording); listing the Workspace section no longer writes on every read (only an object without a row yet gets one); the owner lookup moved from the command into `add_favorite`; the pins' and recents' labels load each object's definition once; the folder row in the customise mode lost its emoji; the empty hints use the shared ghost text; the UI patterns skill now says the toolbar's field is the meetings search. Noted and left: the page actions reach their page as window events (a decision in D-064; URL state would want a `deleted` search param); the ▲ ▼ ◉ ◌ ✎ glyphs are text, not SVG icons.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Rich text, Notes and Tasks, files · release 0.10.6
_2026-11-01_

## What changed
- **The editor** (D-063, TipTap — the one dependency this track adds). A note's or a task's body is a block editor on its page and in the side panel: headings, bulleted and numbered lists, to-do lists, quotes, code blocks, tables, pictures (picked in the dialog or pasted — kept as the record's files), links, bold / italic / strikethrough, and **@** mentions of any record (a chip that names it). It saves as you write (and when you leave it); the words of the body reach search, the agents and the table's cell (as an excerpt — a body is edited on the page, not in a cell).
- **Notes and tasks on a record**: the Notes and Tasks widgets show each one's excerpt; a note's page has the body across the width, the Timeline below, a **Related to** tab — one row per record it is attached to (Twenty's targets: a person, a company, an opportunity, a custom object — add a row and pick) — and **Files**. A task's page the same, with its status, due date and assignee.
- **Files** on every record (a person, a company, a note, a custom object…): drop files on the widget or **Add file…**; a picture shows as a thumbnail; **⋯** opens the file, reveals it in Finder, saves a copy where you choose, or deletes it (to Deleted records). Files are copied into the workspace folder (`files/`), so a backup of the folder has them.
- **Tasks**: the CRM's Tasks object comes with Twenty's **By status** board (made once); a task made from a summary's action item — by the summary's own pass or Add to Tasks — now has its timeline: "Made the task · from the meeting …" with the meeting linked; a typed task too. The Tasks page keeps its list and points at the CRM's table and board.
- No migration. New workspaces get a `files/` folder; existing ones get it at the first file.
- Checks: 170 Rust tests pass (three new: the words of a document; a file lands in the workspace and on the record, a pasted picture too, Save as copies it, a soft delete hides it; names are kept safe); clippy, tsc and the localhost check are clean. Seen in the dev app (light, and the note in dark): on Mr. Tom's Notes tab, "Pricing call notes" typed into "Add a note…" made the note; its page showed the body editor with the formatting row, a heading, a paragraph with the **@Marina Center** mention chip and bold text, a table with its header row, a to-do list with a ticked item, and the picture from the workspace's files; text typed into the editor was in the database a second later; **Related to** listed the person and the company; Marina Center's Notes tab listed the note with its excerpt ("Pricing call notes Call with @Marina Center about the leasing fees fo…"); Mr. Tom's Files tab showed the SVG as a thumbnail and "Term sheet.txt" with its mark, and ⋯ offered Open, Reveal in Finder, Save as… and Delete; "Add to Tasks" on an action item of "Building defects review" made the task, whose page shows the body editor, Due date, Status "To do" and Assignee, and whose timeline row in the database reads "task.created · meeting: Building defects review". After two fixes from that check: a note's Related to table shows the Person, Company and Opportunity columns of its targets (a system junction's relations — the table hid them as system fields), with Mr. Tom and Marina Center as chips and "Add a note target…"; and the Tasks tab's view list now has **By status** — To do (4) · In progress · Done · No value with the cards — seeded once per workspace (a setting remembers) rather than only when the object had a single view.
- Review: the architecture review found six must-fixes, all in: the opener permission had no path scope, so Open would have refused every file (now scoped to the workspace's files folder); the drop point was scaled by the device pixel ratio although macOS hands it over in points (drops on a Retina display would never have landed); two `https://` literals in the editor tripped the localhost-only check (the scheme is kept apart from the host); the editor called the bridge's commands directly (now `addAttachment` / `addAttachmentBytes` in the files bridge, and the Files widget refreshes after a picture); a hard-coded white on the mention list (now `--on-focus`); and the autosave baseline — a blur without a change, or an untouched plain body, would have written the document and left a timeline row (the baseline is what the page shows, and an outside change is followed by comparing documents). Taken too: the editor is keyed by the record, so ‹ › never carries a body across records, and what the debounce holds is saved on leaving; file copies are async; a pasted picture is capped at 20 MB (it travels as bytes); an attachment's path is served only from inside the workspace's files folder (a row made by hand pointing elsewhere serves nothing — tested); the monospace stack is a token; the redundant opener permission and the unused link package are gone. Noted and left: search resolves labels one query a hit (fine on local SQLite).

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

## Env / config
- Dependency added: `@tiptap/react`, `@tiptap/pm`, `@tiptap/starter-kit`, `@tiptap/extension-{image,table,task-list,task-item,mention,placeholder,link}`, `@tiptap/suggestion` (MIT). Asset scope: `$APPDATA/workspaces/*/files/**`. Capabilities: `dialog:allow-save`, `opener:allow-open-path`, `opener:allow-reveal-item-in-dir`.

---

# Last Build — Data model settings: custom objects, fields, relations · release 0.10.5
_2026-11-01_

## What changed
- **Workspace → CRM → Data model** (D-060, Twenty's Settings → Data model). The objects list — Active, Deactivated or All; Standard and Custom chips — and **+ New object** (singular, plural filled from it, an icon, a description, "Skip creating a Name field" for a junction object). A new object is a real table in the workspace's database with Twenty's common columns, a Name field as its label, and the links notes, tasks, files, favorites and the timeline need — so it appears in the CRM's tabs with an "All …" view, a table, filters, sorts, kanban and calendar views, a record page with the Overview, Tasks, Notes and Files tabs, the side panel and search, with no code change.
- **An object's page**: its labels, icon and description (Save); **Identifiers** (the label identifier among its text fields, the image identifier); **Fields** — each row with its type, a relation's target and kind, Required / Unique / Label / Deactivated chips — and **+ Add field**; **Layout** (the 0.10.4 editor on the object's record page); **Status** (Active On | Off; Delete, for a deactivated custom object).
- **The field sheet**: Twenty's type picker (Text, Long text, Rich text, Number, Boolean, Date, Date & time, Select, Multi-select, Currency, Links, Domain, Emails, Phones, Address, Rating, Relation, Array), then the form — label, description; a select's options (label, one of Twenty's ten colours, ▲ ▼, ×, + Option) and default; decimals; the currency; the phones' default country; a text on several lines; a relation's object, type (many to one, one to many, many to many — the last through a junction object made for you) and the field's name on the other object, the junction toggle; Required, Unique. On an existing field: Deactivate / Activate, and Delete once deactivated (custom fields only; standard ones deactivate, as in Twenty). A new field is a real column at once; deactivating keeps everything; unique is refused while duplicate values exist.
- **Relations on both sides**: a many-to-one here makes the one-to-many on the target (and the other way round); a many-to-many makes the junction object "This Target" with a many-to-one to each side and a one-to-many on each side; the target objects' pages list them too.
- **Repair at open**: whatever the metadata says and the schema lacks is created at every workspace open, so an object interrupted half-way is finished next time.
- No migration (run-time DDL; the standard tables are untouched). Columns that point at a custom table carry no foreign key (the reason is in D-060); the engine's own checks and `delete_object` keep the links straight.
- Checks: 167 Rust tests pass (two new: labels become names; a custom object with a select, a relation to Companies and a many-to-many to People lives through the engine — records made and filtered, a field renamed, unique refused on duplicates, the far side deactivated with its relation, the object switched off and on, deleted, and a dropped table brought back by repair); clippy, tsc and the localhost check are clean. Seen in the dev app (the settings in dark, the object's pages in light): Workspace → CRM listed the standard objects; **+ New object** made Partnerships (singular, plural filled from it, icon, description, the junction toggle) and opened its page — labels, Identifiers with Name as the label, Fields, Layout, Status; **+ Add field** showed the type grid, the Select form (options with colours, ▲ ▼ ×, + Option, Default, Required / Unique) and the Relation form (Object, Relation type with its hint, the field name on the other object); the database held `custom_partnership` with `name`, `stage` and `company_id`, the four fields, and Companies' new `partnerships` relation. In the CRM: the Partnerships tab with "All Partnerships", a record added from the Add row, a "Pipeline" kanban by Stage with the record in Lead (the required select's default) beside "No value", and its page (Fields with Stage as a chip, the Timeline's "Made the partnership · Tien Nguyen"). After the fixes: "All Partnerships" holds the three columns in the database, Marina Center's page grew a Partnerships tab from the new relation, and the object page's Fields section listed Name · Required · Label, Stage · Select · Required, Company · Relation → Companies · many to one, Partners · Relation → Partnership Persons · many to many. Deactivating and deleting the object and its junction through the engine left no custom tables and no dangling relation on Companies.
- Review: the architecture review found one must-fix, in: a field label whose column name matched a column another field uses — or a base column such as `created_by_name` — would have aliased it silently; the engine now refuses it, and on a standard table any column the migrations made. Taken too: every column name is checked before the metadata row exists (a field could otherwise be left without a column and stop the open-time repair); that repair logs one object's trouble and goes on instead of stopping the record engine; identifier guards on the link columns and the delete path; the one DROP only ever runs on a `custom_` table; a many-to-many checks its far name before making the junction, and a relation to the same object needs two names; Vietnamese labels fold to names ("Đối tác" → `doiTac`) instead of losing letters; a select's default must be one of its options (by value or label); no fields on system objects; the relation kind is a typed enum in the bridge; a stray transaction and dead lines removed; the helpers of the standard declarations stay private. Found in the dev check and fixed: a field made in the settings joins every view of its object as a visible column at once (it only joined "All …" as a hidden one at the next open); the default record page layout stays computed from the metadata until the owner saves one, so a relation added later has its tab; the field sheet's "+ Option" uses a functional update.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Record pages, page layouts, side panel · release 0.10.4
_2026-11-01_

## What changed
- **Record pages** (D-062, Twenty's). Every object's label now opens its record at `/crm/<object>/<id>` (the old `/crm/people/<id>` and `/people/<id>` links still work). The page: the eyebrow names the object; the title is the label, edited in place (click it; a person's first and last name); **All <objects>** back to the list; **‹ ›** to the previous and next record in the list's order; **⋯** with *Edit page layout* and *Delete* — a deleted record shows a banner with *Restore* and *Delete forever*; then the layout's tabs and, on each tab, its widgets on a 12-column grid.
- **Twenty's default page per object**, made at first visit: *Overview* (Fields beside Timeline), one *Related* tab per one-to-many relation (a company's People and Opportunities; a person's Linked opportunities), *Tasks*, *Notes*, *Files*; a person also gets *Commitments* and *Meetings* (the 0.9.1 person page's sections, now widgets — the old page is gone). The widgets: **Fields** — label · value rows, each edited in place like a table cell, "Not yet captured" when empty (the person's old Notes textarea is now a Notes field); **Related records** — a table of the records on the other side of the relation, editable (cells, and an Add row whose new record is linked back) unless *Allow editing* is off; **Timeline** — made, every change with its before → after per field, deleted, restored, by whom and when; **Tasks** and **Notes** — the ones targeting the record, with status and due date; "Add a task…" / "Add a note…" (Enter) makes one and links it; **Files** — the record's attachments (uploads come with 0.10.6); **Rich text** — a block of text set in the editor. Chart, Emails, Calendar and iFrame are listed in the editor but disabled until their releases.
- **The layout editor** (⋯ → Edit page layout): the page itself in an edit state — drag a widget by its ⠿ handle, resize it by its corner (both snap to the grid; a widget pushed into another moves it down), *Settings* on a widget (title; the fields shown; the relation and Allow editing; the text), *Remove widget*, **+ Widget**; the tab bar: rename the active tab in place, ‹ › to move it, *Default*, × to remove, **+ Tab**; **Save** writes the whole layout at once, **Cancel** drops the changes, **Reset to default** (two clicks) brings Twenty's page back.
- **The side panel**: when a view's ⋯ Options → *Open record in* is *Side panel* (the default, as in Twenty), a label, a kanban card or a calendar event opens the record in a glass panel on the right — the label, the Overview's fields edited in place, **Open** for the page, × or Esc — without leaving the list. *Record page* opens the page instead.
- Migration 0023 (page_layout, page_layout_tab, page_layout_widget), reviewed by the migration-safety agent: safe; its index nit taken, and two readers can no longer make the same object's layout twice.
- Checks: 165 Rust tests pass (two new: a layout made, saved — a widget clamped into the grid, the chosen default tab — and reset; a task added on a company targets it); clippy, tsc and the localhost check are clean. Seen in the dev app (dark, and a person's page in light): clicking HD Bank in the Companies table opened the side panel (the fields as "Not yet captured" rows); its tagline edited there and Enter saved it; **Open** showed the page — Overview with Fields beside the Timeline reading "Changed · Tien Nguyen — Tagline: — → Digital banking partner" and "Made the company · New Dawn"; the People tab listed Mr. Trong, and "Test Person" typed into its Add row was made with the company link (then deleted for good); ⋯ → Edit page layout showed the edit bar, the tab bar and the handles; dragging the Fields widget's corner two columns wider opened its Settings card, Save kept it (the database holds 9 columns; the Timeline moved below), and the page showed it again after leaving and coming back; ⋯ → Delete showed the banner with Restore and Delete forever, Restore cleared it (both in the timeline). In light: Mr. Tom's Overview (Company as a chip, Notes as a field), Commitments (a waiting card with Confirm / Dismiss, the other two groups as ghosts), Meetings ("Building defects review · Involved"), Tasks (ghost rows and "Add a task…"), and the side panel on the People table.
- Review: the architecture review found no must-fixes. Taken: the handle's size and the editor's spacing on tokens; the owner's actor helper moved from the commands into the kernel (`records::actor_of`); the editor snaps to the grid's own rows and gap; Escape inside a field's editor no longer closes the side panel; a clearer "needs a title" error. Noted and left: ‹ › read every id of the table (fine for a personal CRM); a saved layout gets new tab ids, so a `?tab=` link from before the save falls back to the default tab.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

## Env / config
- Migration 0023 (workspace).

---

# Last Build — Kanban and calendar views · release 0.10.3
_2026-11-01_

## What changed
- **Kanban views** (D-061, Twenty's board). **+ Add view** now asks for the type — Table, Kanban, Calendar — and, for a kanban, the select field whose options are the columns (Opportunities by Stage, Tasks by Status…). A column shows its chip, its count and, when set, an aggregate of a numeric field (⋯ Options → Aggregate: Count, Sum, Average, Min, Max of Amount — Twenty's expected amount). Cards show the view's visible fields (Compact view hides them); drag a card to another column to change its field (the timeline records it), or within a column to reorder it; "+ New" at a column's foot adds a record in that column. ⋯ Options → Columns picks the field, their order and hidden ones, as a table's groups.
- **Calendar views**: records placed by a date or date-time field (Tasks by Due date, Opportunities by Close date), by month (Monday to Sunday) or week; ‹ › and Today; drag an event to another day to move it (a date-time keeps its time); "+" on a day adds a record on that day. ⋯ Options → Date field changes the field.
- Filters, sorts, search and Deleted records apply to all three view types; **Save as new view** keeps the type and its settings.
- No migration: the view rows of 0.10.2 already carry the type and the fields.
- Checks: 163 Rust tests pass; clippy, tsc and the localhost check are clean. Seen in the dev app (dark): "+ Add view" → "Pipeline", Kanban, Stage made the board — New · Screening · Meeting (2 · 150,000) · Proposal with the cards' fields, after ⋯ Options → Aggregate: Sum of Amount; dragging "HD Bank API pilot" from Meeting to Proposal changed its stage (the timeline holds the diff) and placed it last in the column; "+ Add view" → "By due date", Calendar on Tasks showed November with the two tasks on their days and today marked.
- Review: the architecture review found three must-fixes, all in: choosing "None" for an aggregate, a column field or a date field now clears it (a `null` in the patch was ignored); a calendar on a date-time field asks for the Mac's midnights as instants, so early-morning events no longer slip off the first day; drops no longer let the browser paste a card's link into an input. Taken too: a card dropped on itself or an event dropped on its own day writes nothing; with a sort on, cards keep the sort's order (no manual reorder, as Twenty); changing a kanban's column field forgets the old field's group order; an aggregate without a field is only Count; cards hide empty fields; two ghost cards in an empty column; drop targets use the system accent ring; spacing on tokens; cards take the keyboard; invalid ARIA roles dropped; the unused `columns` prop of the calendar removed.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

---

# Last Build — Table views: filters, sorts, groups, saved views · release 0.10.2
_2026-11-01_

## What changed
- **Views** (D-061). Every CRM object now has Twenty's view bar: the view dropdown on the left ("All People" by default; + Add view; drag to reorder; ⋮ Edit, Duplicate, Delete), **Filter**, **Sort** and **⋯ Options** on the right, and the active rules and sorts as chips underneath.
  - **Filter**: pick a field, then the operator its type allows (text: contains, doesn't contain, empty, not empty; numbers and amounts: is, is not, ≥, ≤; dates: is, relative (past / next / this day · week · month · year), in past, in future, today, before, after or equal; selects: any of the options; relations: pick records; true/false) and a value. Rules in the bar combine with And; **Advanced filter** adds rule groups — And, Or, Not — nested up to three deep, shown as an "n advanced rules" chip.
  - **Sort**: ascending or descending, several fields; also from a column's ⋮ menu.
  - **Options**: Fields (show, hide, drag or ▲▼ to reorder), Group (a select field; A→Z, Z→A or manual order; hide groups), Compact view, Open record in (side panel · record page), Visibility (workspace · unlisted — not on the default view).
  - **The table**: drag a column's border to resize, drag a header to reorder; a column's ⋮ offers sort, filter and hide; grouped tables show a collapsible header per group with its count and aggregates and an Add row per group (a new record takes the group's value); the **aggregate footer** — hover a column and choose Count, count empty / not empty / unique, percent empty / not empty, Sum, Average, Min, Max (amounts in units).
  - **Unsaved changes**: everything above edits a working copy; the table follows at once; **Update view** saves it, **▾ Create view** or **Save as new view** (on the default view) makes a new one, **Reset** drops the changes.
- Migration 0022 (views and their parts; a favorite can be a view), reviewed by the migration-safety agent — its two indexes added.
- Checks: 162 Rust tests pass; clippy, tsc and the localhost check are clean. Seen in the dev app (dark, and the components of 0.10.1 in light): "Job title contains Manager" from the Filter menu narrowed People to three and showed its chip; Sort → Descending → Name added its chip and reordered the rows; **Save as new view** made "Managers", selected it, and its rule and sort are in the database (they survive a relaunch); Opportunities grouped by Stage showed New · Screening · Meeting (2, 150,000) · Proposal (1, 45,000) with an Add row per group, and the footer's Sum of Amount read 195,000.
- Review: the architecture review found three must-fixes, all in: a rule just added (no value yet) no longer breaks the table — it is left out of the query until it has one (both the page and the engine's own view query skip it, and an empty rule group is skipped); **Update view** is now one transaction (a `save_view` command) instead of five writes, so a failure changes nothing and the working copy no longer resets mid-way; the column ⋮ menu is no longer clipped by its header. Taken too: the two table menus close on Escape or a click outside; the resize handle takes the keyboard (← →); Compact view now shortens the rows; Sort offers a full name's first or last name (and other composite parts); relation and amount rules query the same way from a saved view as from the bar; Sum and Average are only offered on numbers and amounts; an empty group names itself; a read error shows its reason; radii and a width moved to tokens and classes; a Rust test covers a nested rule group through save, duplicate and the engine's query (163 tests).

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want.

## Env / config
- Migration 0022 (workspace).

---

# Last Build — The record engine: Twenty's data model, five standard objects, a first table · release 0.10.1
_2026-11-01_

## What changed
- **The record engine** (D-060, `kernel/records/`). Twenty's data model lives in SQLite now: `object_meta` and `field_meta` describe every object and field; the standard objects — People, Companies, Opportunities, Notes, Tasks, and the system objects behind them (note and task targets, the workspace member, attachments, favorites, timeline activities) — are declared in Rust with every one of Twenty's standard fields and synced into the metadata at every workspace open. Every object is a real table; a composite field (full name, emails, phones, links, address, currency, actor) is a few columns side by side; a relation is a `_id` column.
- **One query layer**: lists with Twenty-shaped filters (AND/OR groups, the operators of each type incl. relative dates), sorts, search and pages; create, update (with the diff), soft delete, restore, delete for good — each validated per type, each writing the record's timeline and keeping the FTS5 search index in step.
- **CRM → tabs**: People, Companies, Opportunities, Notes, Tasks — each a Twenty-style table: one column per field, click a cell to edit it in place (text, numbers, dates, selects with colours, currency, links, emails, phones, address, rating, a picker for relations, first/last name), tick rows for Delete, an Add row at the bottom (Enter), a search box, and **Deleted records** with Restore and Delete forever. Commitments (0.9.1) stay as the last tab.
- **Your people carried over**: names split into first and last, organisations became Companies (linked), the owner is the one workspace member; the people memory (Workspace → People) keeps working and writes both shapes. Tasks gained Twenty's status (mirrors done); the Tasks page is unchanged.
- Migrations 0020 (metadata, timeline, search index) and 0021 (the tables and columns), reviewed by the migration-safety agent — its must-fixes are in (status triggers, a cleanup trigger for people deleted by the memory).
- Checks: 162 Rust tests pass; clippy, tsc and the localhost check are clean. Seen in the dev app, light and dark: the People table with the migrated names and companies; an opportunity added from the Add row, its stage changed in place (the timeline shows the diff), its company picked from the picker, deleted, listed under Deleted records, restored; a company's tagline edited in place and found by search.
- Review: the architecture review found three must-fixes, all in: the search index now syncs at every open and after the people memory writes (it only indexed once before); a system object's permanent delete can no longer wipe the timeline; two concurrent first opens of a workspace can no longer carry people over twice (the hub opens one workspace at a time). Taken too: table and column names are checked before they reach SQL (the DDL layer of 0.10.5 builds on it); a required field without a value becomes an empty record, as in Twenty, instead of a database error; a date filter on a day no longer slips a day west of UTC; a comparison filter without a value says so; a cleared required select returns to its first option; a task done keeps its first done time; deleting a company for good also clears its people's organisation text; a merged person keeps the company link; the relation picker reuses the menu styles; `fold` moved to the kernel.
  Known gaps, by design for now: ticking a task's status in the CRM table doesn't tick the action item in its summary (the Tasks page does — 0.10.6 reconciles them); tasks added from meetings reach the CRM search at the next open; the CRM can create a second person with the same name (the people memory can't) — the record page's merge comes with 0.10.4.

## Manual steps
- None. Nothing is installed; update from Settings → About → Check for updates when you want. The first open of each workspace runs the two migrations and the carry-over (a second or two).

## Env / config
- Migrations 0020 + 0021 (workspace). Cargo: the `specta` crate gains the `serde_json` feature. Records cross the bridge as `Json` (`unknown` in TypeScript).

---

# Last Build — Modules: Projects and CRM, switched per workspace · release 0.10.0
_2026-11-01_

## What changed
- **Modules** (D-059). Workspace → Overview has a new **Modules** group: **Projects** and **CRM**, each On | Off. Off hides the module from the sidebar (its pages open Meetings instead), pauses its agents — the Projects agent after a summary, the commitments pass, the Email reading's project updates and commitments — and leaves it out of Today. Its data stays; On brings everything back. Both are on in every workspace; the setup wizard's first step has the same two rows.
- **People → CRM.** The sidebar item is now **CRM** (`/crm`; a person at `/crm/people/…`); the old `/people` links redirect. The page itself is unchanged in this release — it becomes Twenty's CRM over the next releases.
- **Plan v4** is the new `docs/plan.md` (D-058): the Twenty CRM and the Plane project manager rebuilt inside New Dawn, release by release, with the parity tables that say what is the same, adapted or left out. Plan v3 is `docs/plan-v3.md`.
- Checks: 161 Rust tests pass (the new one covers the default and the round trip of the setting); clippy, tsc and the localhost check are clean.
- Seen in the dev app, light and dark: the Modules group under the workspace facts; CRM off → the sidebar item gone, Today showing VIFC's project updates and no commitments, `/people` landing on Meetings; CRM on → the item back with its badge, `/people` landing on the CRM page.
- Review: the architecture review found one must-fix — the person page still used the old route id and would have thrown on open — fixed and seen in the dev app. Its other points were taken too: the meeting menu's "Find project updates" is gone while Projects is off; the Email reading leaves the projects out of the prompt when Projects is off (no wasted model time); a module's page shows nothing for the moment the switch is read instead of flashing; the wizard keeps your Off choice if a later save fails. Left as is: Today and the sidebar still read a paused module's rows before blanking them (harmless).

## Manual steps
- None. Nothing is installed (the installed app is untouched); update from Settings → About → Check for updates when you want.

## Env / config
- No migration: the choice is the workspace setting `modules` (`{"projects":true,"crm":true}`; missing = both on).
- The unattended run for plan v4 is on (state in the memory file `phase3-autorun.md`); it keeps the disk ≥ 4 GB free by clearing build caches.

---

# Last Build — Today: what needs you, across your workspaces · release 0.9.3
_2026-10-31_

## What changed
- **Today** (sidebar, first item, D-057). One page shows what needs you in every workspace:
  - **Due today**: your open tasks due today or earlier. Overdue ones are in red.
  - **Waiting for you**: the project updates, commitments and email tasks waiting for your decision, with links.
  - **Projects to watch**: active projects whose next milestone is due within a week, or is overdue.
  - **Commitments to follow up**: what others promised you and you confirmed.
  - **This week's meetings**: Monday to Sunday.
- With more than one workspace, each row names its workspace. Clicking a row from another workspace switches to that workspace, then opens the page.
- Today only reads; nothing new is stored. Meetings is still the screen New Dawn opens on.
- If a workspace can't be read, Today names it at the top instead of leaving it out silently.
- Checks: 160 Rust tests pass. The new one covers what Today lists and leaves out: tasks due today or earlier, projects due within a week (soonest first), this week's meetings (Monday counts, next Monday doesn't) and the current workspace first.

  Clippy, tsc and the localhost check are clean.
- Seen in the dev app, with a second workspace (Majoris Digital):
  - Today listed a task from each workspace, the overdue one in red, a project due on Tuesday, the waiting proposals, a confirmed commitment and this week's two meetings.
  - Clicking the Majoris Digital task switched to that workspace and opened its Tasks.
  - Light and dark both look right.
- Review: the architecture review found no rule violations and no must-fix. Its other points were all taken:
  - a workspace that can't be read is named;
  - sections no longer say "Nothing…" when the page couldn't be read;
  - a failed switch shows why;
  - Today refreshes as soon as you switch or rename a workspace;
  - rows show a keyboard focus ring.

## Release
- **v0.9.3** is published on GitHub. **Your installed New Dawn wasn't touched.** Update it from Settings → About → Check for updates when you're ready.

## Manual steps
None.

---

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
