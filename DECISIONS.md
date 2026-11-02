# Architectural Decisions

Newest first. Every entry: what was decided, why, and what it means for future work. Keep entries short; link to `docs/` for the long version.

## D-067 — Dashboards: a record with a layout of its own, one chart query on the engine, charts as inline SVG
_2026-11-02_

**Context** (plan v4 §3.6, 0.10.10): Twenty's dashboards are records of a Dashboard object whose page is tabs of widgets on a grid — Bar, Line, Pie and Aggregate charts (a source object, a filter, the X field with a date granularity, the Y aggregate with a ratio, a group by, sort, omit zero, cumulative; stacked, data labels, legend, axis names, prefix and suffix), a Table of a view with "Allow editing", Rich text and an iFrame — capped at 100 bars and 50 groups; a Chart widget of related records on record pages.

**Decision — a dashboard is a record** of the standard object `dashboard` (migration 0027: its table, `page_layout.dashboard_id`, `favorite.dashboard_id`), so views, favorites, ⌘K's "Create a dashboard…", the sidebar's Workspace section, search and the data model come free; its record page *is* the dashboard — the tabs of its own page layout (0023's `page_layout` with `kind = 'DASHBOARD'` and `object_id` empty, made with one empty tab the first time it is asked for) in view mode, and the 0.10.4 layout editor with the dashboard palette (Chart, Table, Rich text, iFrame) behind ⋯ → Edit dashboard; "Duplicate dashboard" (⋯ and ⌘K) copies the record and every tab and widget. The side panel and the table show a dashboard like any record (its name).

**Decision — one chart query** (`kernel/records/charts.rs`): the widget's config names the object, a view-style filter (the same rules the view bar keeps; on a record page the relation whose records are charted, filtered to the record), the X field (a select's options in their order with their colours — every option, zeros kept unless omitted —, a date bucketed by `strftime` per granularity (a date-time in this Mac's own time), a relation by its target's label, a boolean as True / False, text as is), the Y aggregate (the engine's own operations plus Count true / false and a Ratio; Twenty's Earliest / Latest wait for a chart that can show a date), an optional second dimension that makes series; one `GROUP BY` through the engine's filter SQL, every value bound, names from the metadata; sorting, cumulative sums and the omission of zeros on the result; capped at 100 categories and 50 series with a note. Amounts come back in units. The chart's query key starts like the records' queries, so what the UI already refreshes on a record change refreshes the charts (a card moved on the kanban moves the bar).

**Decision — charts are inline SVG** (`src/kernel/charts/`), no library: bars (vertical or horizontal, grouped or stacked), lines, a ring with the total in its centre, a large value with prefix and suffix; the Majoris families as `--chart-1…8` tokens with dark variants, a select's option colours mapped to the same families; DM Sans labels, four round ticks, a legend when there are series. The iFrame widget shows only a `localhost` address (checked in the widget, and the CSP's `frame-src` allows only localhost), rule 3 kept.

**Means for future work**: workflows (0.10.11) get dashboards' "run a manual workflow" entries; the CRM agent (0.10.15) reads a chart's data for its answers; Plane's analytics (0.11.10) draw with the same chart components; the gauge Twenty plans is a fifth kind when they ship it.

---

## D-068 — Workflows: records with versions, a run that keeps its place, record events from the engine
_2026-11-02_

**Context** (plan v4 §3.7, 0.10.11): Twenty's workflows — a trigger (a record created / updated (fields watched) / created or updated / deleted; by hand, everywhere or on one or many records; on a schedule; a webhook) and steps (Create, Update, Delete, Search, Upsert, Filter, If/else, Iterator, Delay, Form; later email, code, HTTP, AI), `{{stepId.field}}` variables, versions (Draft · Active · Deactivated · Archived — Activate publishes the draft and archives the live one; "Use as draft"; Discard), runs with each step's output, Retry, a Workflow Runs object.

**Decision — a workflow is a record** of the standard object `workflow` (name, status) and a run a record of `workflowRun` (name, status, workflow, started, ended), so views, favorites, ⌘K's "Create a workflow…", the sidebar's Workspace section and search come free; a workflow's page *is* the builder and a run's page its steps. The versions, runs' state and triggers are the engine's own tables (migration 0028: `workflow_version` holds one JSON document a version — the trigger and the ordered steps, an If/else's routes and Else and an Iterator's inner steps nested; `workflow_run` keeps the trigger's data, the context (every step's output by id), the steps done, `wait_until` for a delay, `waiting_for` for a form, the error; `workflow_automated_trigger` is rebuilt from the active version on activation — RECORD with its object, event and fields, SCHEDULE with `next_at`, WEBHOOK shown only until the local API).

**Decision — the engine tells, it never calls**: `kernel/records/events.rs` is one bounded channel of record events (object, event, before, after, the actor's source) emitted by the engine's create, update and delete after the timeline row; `modules/workflows/triggers.rs` subscribes once at start-up and matches the active RECORD triggers (the object, the event, a watched field that changed); a workflow's own writes (actor source WORKFLOW) and an import's rows aren't announced — no loops, and a large import doesn't overrun the bound (a batch event per import is the upgrade when a workflow should answer imports); a full channel drops with a warning rather than block the engine. Workflows and their runs are never a trigger's object nor a step's. Schedules and delays are one task a minute over the open workspaces: a due schedule starts one run however many ticks were missed (a launch catches up without a flood), the delays over go on. A workflow in Deleted records answers nothing until restored. A run that a quit or a crash stopped mid-step is marked failed at the next launch ("Stopped when New Dawn quit — Retry goes on from this step") rather than running a half-done step twice by itself.

**Decision — a run keeps its place in the database**: `run.rs` walks the steps in document order, skipping the ids done, and writes the context and the done list after every step, so a run goes on after a quit — a Delay leaves it WAITING with `wait_until` (the scheduler resumes it at its own time), a Form leaves it WAITING for the owner (the run's page asks; the answers go into the context under the step's id). Templates read the context by path (`trigger.record.pointOfContact.id`, `s1.name`, `iterator.currentItem`, `today+3d`, `now`); one template alone keeps the value's type, text with templates becomes text. Conditions (Filter, If/else) are evaluated in memory on the resolved values; an If/else picks its route once (a run that resumes inside it stays in it; a route without conditions never holds); an Iterator clears its inner steps' outputs between items (a delay waits and a form asks for each); a Filter that doesn't hold ends the run — inside an Iterator, that item only. A run claims its status to go on — a start from NOT_STARTED, a resume or a form's answer from WAITING, a retry from FAILED — so a cancelled or finished run never comes back; Cancel works while a run waits. What the owner types can't crash the app: a date or a wait off the calendar is refused or empty. Record steps go through `query::create / update / delete / list` with the actor named after the workflow, and refuse a new record whose title or name came out empty (the engine itself allows an untitled "+ New"); a record a step made isn't made again on Retry; Upsert matches by the import's identifiers (`transfer::find_by`); a new task or note is attached to records through the engine's junctions. A manual run is the active version (the draft for Test), once a record for Single, once with them all for Bulk.

**Decision — the builder is a list, not a canvas**: the trigger first, then the steps as cards, routes and inner steps indented; the chosen card's settings on the right, typed by the step (an object, its writable fields each with the control the field wants, targets for a task or note, a search's rules from the view bar's operators, conditions, a delay, a form's fields), every text setting with a picker of the variables available at that point (the trigger's record and, for an update, the record before it; the steps that have run by then — never a sibling route's; the iterators it sits in; dates); the draft is saved as you go and when the page closes (reading a workflow never makes a version — the first edit does; Activate and Test check the definition); Test (with a record when the trigger wants one), Activate, Deactivate, Discard draft (never the only version); the versions and the last runs under the settings. Workflow runs aren't in search (a schedule makes one a minute; the workflow's page lists them). ⌘K lists "Run <workflow>" for the global manual ones and "Run <workflow> on this record" on a record's page; the record page's ⋯ lists the same.

**Means for future work**: 0.10.12 adds the steps that leave the engine (Send email through Mail, Code, HTTP to localhost, the AI agent on the local model) as more `run_step` arms; 0.10.14 gives a WEBHOOK trigger its URL on the local API and the Bulk entry to the table's selection bar; Plane's automations (0.11.2) reuse the runner on work items; the record events feed the live refresh and the agent's "what changed" later.

---

## D-069 — Workflows' outside steps: the local model, a Mail draft, this Mac's addresses, code in the window
_2026-11-02_

**Context** (plan v4 §3.6, 0.10.12): Twenty's AI Agent (an agent and a prompt; a structured output), Send Email and Draft Email (To, Subject, Body from a connected account), HTTP Request (URL, method, headers, body; 30 s, 10 MB) and Code (`export const main = async (params) => …`, its return the output, a Test button) — each adapted to a Mac with no cloud.

**Decision — the AI step goes through the gateway**: the prompt, its variables resolved, on the local model (the app's, or one picked from Ollama's installed models) through `kernel/ai/ollama::session` in the foreground lane — the queue, Whisper's unload and `ai_log` (agent `workflow`) as for every AI job (rule 2). With outputs (a name and text / number / true or false), the model is asked for one JSON object with those keys and each becomes `{{step.<name>}}`; without, the answer is `{{step.text}}`. An answer that isn't the JSON asked for stops the run with the words it gave; a record or a list in the prompt goes in as JSON. One workflow AI step waits in the queue at a time, so parallel runs don't stack foreground jobs ahead of a meeting's summary. The module keeps the app's handle in a cell set when the triggers start (a run has only its workspace's pool otherwise). Named agents with tools join in 0.10.13.

**Decision — email is a Mail draft, never sent by New Dawn**: the step composes To (an address, or several), Cc, Subject and Body from their templates into a `mailto:` link kept as the step's output; the run's page shows it with **Open in Mail**, and the owner presses Send in Mail. New Dawn never drives or launches Mail by itself (D-066), so a scheduled run at night leaves drafts waiting, not mail sent. A To with no address stops the run plainly.

**Decision — HTTP to this Mac only**: GET, POST, PUT, PATCH, DELETE to `localhost`, `127.0.0.1` or `[::1]` over http(s); any other host is refused before a byte leaves ("Only this Mac's addresses (localhost) can be called: … is on the network") — at Activate when the address is written out, at the step when it comes from a variable. No proxy is used (a system or environment proxy would carry a "localhost" call off the Mac) and no redirect followed; Ollama's port is refused (the model is reached through the AI step and its queue, rule 2); 30 s; 10 MB, of which 1 MB is kept in the run; headers and a body with variables — a body written as JSON is resolved as JSON, so values are escaped and a lone `{{x}}` keeps its type; the answer's JSON (or text) and status are the output; an answer that isn't 2xx stops the run with its status and first words. Rule 3 holds: the address is the owner's, the check is the app's.

**Decision — code runs in the window**: the step's JavaScript runs in a Web Worker made from it for one run — `export const main = async (params) => { … }` given the step's inputs (templates resolved), its return value (JSON) the output; the worker has no DOM, the window's policy allows it no connection but the app's own (`worker-src blob:` added, `connect-src` unchanged), fetch, XMLHttpRequest, WebSocket, EventSource and importScripts are removed before the code runs, and it is stopped after 10 s. The run waits for the window (`waiting_for` the step): the window asks for the waiting code steps of the current workspace every 5 s while it is visible and when it comes to the front, runs each, and answers; with the window closed the run waits, and its page says so. Each wait has an attempt token the answer carries back, and an answer goes into the run only once the run is claimed — a late or repeated answer is refused. Nested workers are cut off too; a code step whose attempt was started but never answered when the window comes back (it took the window down) is answered with an error instead of running again. An error stops the run with it; Retry asks the window again. The Test button runs the code with the inputs as typed.

**Decision — slow steps don't hold the others**: each record event's runs and each workspace's minute go on their own task, so an AI step waiting behind a meeting's summary or an HTTP call never holds the next event, schedule or delay up; a schedule's slot is taken by moving its time on, so overlapping minutes start it once. A running run can be cancelled: it stops before its next step (the step under way finishes).

**Means for future work**: 0.10.13's agents become a choice in the AI step; 0.10.14's local API makes HTTP steps between workflows and the webhook trigger's address; the code step stays JavaScript without TypeScript's types (no compiler is bundled).

---

## D-070 — The CRM's assistant: agents as settings, a turn of lookups on the local model, writes only as proposals
_2026-11-02_

**Context** (plan v4 §3.6, 0.10.13): Twenty's AI chatbot (questions over the workspace's data in plain words, follow-ups that keep their context, "this company" meaning the record on screen) and AI agents (a name, a prompt, a model, the tools and permissions they get, usable as workflow steps); here one owner, one Mac, the local model.

**Decision — agents are settings, not records** (migration 0029, `crm_*` tables beside the meeting agents of D-035): an agent is a name, a description, its instructions, its model (the app's or one of Ollama's installed models) and the tools it may use; a default "Assistant" is made the first time it's asked for and can't be deleted (Workspace → CRM → AI agents). A chat thread belongs to an agent (the default when its agent is deleted) and may sit on a record — on a record deleted for good, it stays, detached.

**Decision — one turn, lookups first, through the gateway**: a question opens a session on the AI queue in the lane the owner waits in (the gateway's new `chat` turn: Ollama's `/api/chat` with tools, logged in `ai_log` as a completion, the session's model, Whisper unloaded; thinking off — a chat turn waits on every round, and tool calls read well without it — an exception to the house's "thinking on") and the model answers or calls tools, up to six rounds: `find_records` (filters in the view bar's operators; a select by its label, an amount in units, a relation by the record's name; `inactive_days` — no change, note or task on the record for that long, read from the timeline and the note/task targets over the first 2 000 matches), `count_records`, `aggregate` (sum, average, smallest, largest; amounts back in units), `open_record`. A record may be named rather than given by id; numbers may come as text; `find_records` takes an order. A tool's mistake goes back to the model as an error to correct, never as a failed turn; each call and result is kept in the thread, and an earlier turn's results are read back in brief so a long chat stays within the model's context. A question is kept only once the model can be reached. The system message carries the agent's instructions, today, the ground rules (look things up, never invent; link records as `[label](/crm/…)`; changes are proposals), the objects with their fields and options, and the record the chat is on.

**Decision — the model never writes**: `propose_create`, `propose_update` and `propose_note` only store a proposal (checked as the engine would, shown in words: "Add a note to Keppel Land — “…”"); the panel shows it as a card with **Apply** and **Dismiss**; Apply writes through the engine as the agent (actor AGENT, named after it), once — a second click finds it applied — and a refusal leaves it proposed with the reason. A proposed value is written whole: an amount keeps the record's currency (else the field's), a name is split in two, an email, a phone or a link is its first part. An update never lands on a record deleted since; a note is attached to its record through the note's targets, or not made at all. Workflows, their runs, dashboards and objects turned off are out of the assistant's reach. The record engine's `forget_visits` detaches the threads on a record deleted for good — the one place the kernel writes the assistant's table, a module's.

**Decision — the panel**: ⌘K → "Ask AI" (and "Ask AI about this <object>" on a record), or a record's ⋯ → "Ask AI about this …", opens a glass panel on the right: the chats of that place (newest first; New chat; an agent to pick when there are several), the owner's questions on the right, the answers with their record links opening in the app and the AI marker with the model, each lookup in a quiet line, proposals as cards, the AI progress bar while the model works, and the question box (Enter asks). A workflow's AI step can take an agent: its instructions, its model and its lookups — never a proposal inside a run.

**Means for future work**: 0.10.15's CRM agent runs on these tools; Plane's Projects agent (0.11.12) gets the same proposal cards; chat threads on records can join the record page as a tab when one is asked for.

---

## D-071 — The local API: Twenty's REST on 127.0.0.1, keys kept as fingerprints, webhooks to this Mac
_2026-11-02_

**Context** (plan v4 §3.8 and §3.7 Developers, 0.10.14): Twenty's REST API (`/rest/<objects>` with a Bearer key, cursor pagination, filters, PATCH, batch of 60, generated docs), API keys (name, expiry, revoke, shown once) and webhooks (record events posted to a URL, signed). New Dawn has no server and no network beyond this Mac (rule 3).

**Decision — an HTTP server on this Mac only, off by default**: `axum` (the one dependency plan §5.8 names for it) on `127.0.0.1` — never another interface — on a port the system picks the first time and kept in the registry (a taken port is replaced, and kept); the owner turns it on in Workspace → CRM → Developers, and it starts at launch when on. It serves Twenty's paths and shapes: `GET /rest/<objects>` → `{ data: { <objects>: [ … ] }, pageInfo: { hasNextPage, hasPreviousPage, startCursor, endCursor }, totalCount }` with `limit` (60, ≤ 200), `starting_after` / `ending_before` (opaque cursors), `filter` in Twenty's syntax (`field[op]:value`, commas, `and()` / `or()` / `not()`, `name.firstName`, ops eq · neq · in · containsAny · is NULL/NOT_NULL · gt · gte · lt · lte · startsWith · like · ilike) and `orderBy` (`field[AscNullsFirst]`…); `GET /rest/<objects>/<id>` → `{ data: { <object> } }`; `POST`, `PATCH`, `DELETE` → `create<Object>`, `update<Object>`, `delete<Object>` (a delete goes to Deleted records, as in the app); `POST /rest/batch/<objects>` → `create<Objects>` (60 at most, all read before any is written); errors as `{ statusCode, error, messages }`; `companyId` accepted for the `company` relation. Writes go through the engine as "API key <name>". `/docs` is one page — the endpoints, the syntax, each workspace's objects and fields, curl lines on this port. `createdAt` / `updatedAt` aren't filterable yet (the engine keeps them outside the fields).

**Decision — keys are fingerprints**: a key is `nd_` + 48 hex characters from the system's random source, shown once; only its SHA-256 is kept, with a 10-character prefix to tell keys apart, an optional expiry (30, 90, 365 days) written in the same text as "now", a revoke, and its last use. A request's key is looked up across the open workspaces and names the workspace it reads and writes.

**Decision — webhooks to this Mac, signed like Twenty's**: a webhook is an address on this Mac (the HTTP step's check: localhost, 127.0.0.1, [::1]; no proxy, no redirect, 10 s), the events it wants (`*.*`, `person.*`, `opportunity.updated` — Twenty sends everything; this is a superset), a secret made for it, and on/off. Each record event is posted as `{ event, data, timestamp }` with `X-Twenty-Webhook-Timestamp` and `X-Twenty-Webhook-Signature` — the hex HMAC-SHA256 of `timestamp:body` with the secret (written on `sha2`, checked against RFC 4231). The last 50 deliveries are kept with their status, time and error. The record events (D-068) now have several listeners — the workflows and the webhooks — each with its own bounded channel, each event's work on its own task.

**Decision — the workflows' webhook trigger**: `POST /webhooks/workflows/<workspace>/<workflowId>` (no key, like Twenty's; this Mac only) starts a run of the active version whose trigger is a webhook, the JSON body as `{{trigger.…}}`; the builder shows the address when the API is on.

**Decision — this Mac only, against a browser too**: listening on 127.0.0.1 keeps the network out, not a web page in a browser here. Every request's `Host` must name this Mac (`localhost`, `127.0.0.1`, `[::1]`, any port) — a page reached through DNS rebinding names its own — and a browser's `Origin`, when sent, too; anything else gets 403 `ForbiddenException`. The docs and the workflows' webhook, which take no key, are covered by the same guard. A workflow's HTTP step refuses the API's own port: a workflow writes through its record steps (whose writes start no trigger, D-068), never around them.

**Decision — writes as Twenty's**: a PATCH writes the parts it's sent — a name's `firstName` alone keeps the `lastName`, an email's or an amount's other parts stay. A batch reads and checks every value before writing any; the engine writes each record in its own transaction, so a database failure part-way keeps the records before it.

**Means for future work**: Plane's API (0.11.11) is served by the same server under its own paths; an MCP server over this API stays optional (plan §3.6).

---

## D-083 — Plane's analytics, exports and CSV importer: read where the work lives, Plane's columns both ways, the history already kept
_2026-11-02_

**Context** (plan v4 §4.5, 0.11.10): Plane's Analytics (an overview; work items by state, Created vs Resolved over time, customized insights by priority, assignee, state, labels or cycles; Cycles and Modules tabs that are paid there), its exports (CSV, XLSX or JSON with the current filters, an export history) and its CSV importer (Plane's columns, matching rules, a table of the runs and a JSON summary).

**Decision — analytics read the work where it lives**: counts are queries on the items themselves (no snapshots) — the open projects', or one's; no Intake request, nothing archived. Created vs Resolved counts each day on this Mac's calendar the items made and the ones that entered a completed state (`completed_at`, D-074); imported work items are made on the import's day, so that day counts them all. Customized insights count by state, state group, priority, assignee, label, cycle or module — an item with several labels, assignees or modules counts in each — with how many of each are completed; across the projects a state, label, cycle or module is one bar by its name (each project's Done is one Done). The Cycles and Modules tabs are each one's progress (ours, as the plan says). The page's choices (tab, project, period, axis) are its address; Plane's saved analytics views aren't kept.

**Decision — Plane's columns both ways**: an export writes Plane's twenty columns (Identifier, Name, Description, State Name, State Group, Priority, Assignees, Labels, dates, Estimate, Cycle Name, Modules, Parent, Subscribers, Links, Comments, Created, Updated and Completed At) as CSV, XLSX (a plain one-sheet workbook) or JSON, for what the Work items tab's filters keep, to the file the save dialog names. The importer reads the same columns from a CSV or a workbook (any case): a row is a work item — its state by name, made in its State Group when the project lacks it; labels by name, made when missing; people by name or "You" (others left out and said); dates, estimate, cycle and modules by name; links; comments (one cell, " ¦ " between them) — and parents link by Identifier within the file once every row is an item. Before anything is written, a preview says what the file holds, the states and labels it will make, the rows named like work items the project has already, and what won't come in as it says; the summary says it by row, and an item once made stays made (a comment or subscriber that fails is said, not undone). A CSV cell a spreadsheet would read as a formula (starting =, +, -, @) is written after a ' and read back without it — the CRM's exports and imports too (`kernel/csv.rs`) — and a CSV starts with a byte-order mark so Excel reads its accents. Exports and imports are rows of the transfer history the CRM already keeps (0025 `export_job`, `import_job`, object "pm_item") — no new table.

**Where the owner sees it**: Projects → **Analytics** in the sidebar (hideable), with Overview, Work items, Cycles and Modules; the Work items tab's **Export** menu (CSV, XLSX, JSON); a project's ⋯ **Import work items (Plane CSV)…**.

**Means for future work**: Plane's API (0.11.11) can serve the same rows; the parity audit (0.12.0) checks the columns against Plane's.

---

## D-082 — Plane's Home, Your work and navigation: a Projects section in the sidebar, recents as visits, Power K inside ⌘K
_2026-11-02_

**Context** (plan v4 §4.1, 0.11.9): Plane's Home (greeting with the day and time; Quick links, Recents — All, Work items, Pages, Projects —, Your stickies; Manage widgets), Your work (counts of what you're assigned, made and follow; workload by state group and priority; a pie; recent activity), Power K (search, actions and navigation in one palette), the documented shortcuts, Customize navigation, notification preferences, and the sidebar's shape: Home, Your work, Inbox, Drafts, Projects with their parts, Views, Archives, Favorites.

**Decision — the sidebar gets Plane's Projects section**: when the Projects module is on, the sidebar's top section keeps the app (Today, Meetings, Live, Tasks, CRM, Email, Workspace) and a **Projects** section follows — Home, Your work, Inbox (its unread count), Drafts, Views, Active cycles, Archives, All projects (with the updates waiting for you), then each open project, which opens onto its parts as its features say (Work items, Cycles, Modules, Views, Pages, Intake). Twenty's CRM section (with the Favorites) stays below it. **Customize navigation** (✎ on the section) shows or hides each of the section's items (0041 `pm_navigation`); the projects always show.

**Decision — Home and Your work are pages of the module**: Home's widgets (0041 `pm_home_widget`: on or off, in order — Manage widgets) are Quick links (`pm_quick_link`: an address once, opened in the browser), Recents and Your stickies (`pm_sticky`: rich text and a colour name). Recents are visits (`pm_recent`: a work item, page or project opened, its time moved on each visit, read joined to what still exists — nothing archived, no Intake request, no archived project — and the newest 100 kept). Your work reads the same filters as views (`assignees`, `created_by`, `subscribers` = you, D-077) across the open projects; its bars and pie are drawn from that list, and its recent activity is the owner's own activity rows. Notification preferences per kind already lived in Settings → General (D-075).

**Decision — Power K is in ⌘K**: the one command menu (D-064) also finds work items by key or title, projects, cycles, modules and pages by name (a few of each, open projects and features that are on), goes to the Projects pages, and creates a work item (in the project on screen, else the first) through the create sheet once the menu has closed. Plane's go-to keys take the letters Twenty left: `g h` Home, `g w` Your work, `g i` Inbox, `g v` Views, `g r` Archives, `g a` Active cycles, `g j` All projects (`g p` stays People). **Archives** (`g r`) lists what's archived across the projects — projects, and the open ones' work items, cycles, modules and pages (a sub-page under its archived page is reached through it) — each restored there or opened where it lives.

**Means for future work**: analytics (0.11.10) read the same filters as Your work; the Projects agent (0.11.12) can file its proposals into the Inbox the sidebar counts.

---

## D-081 — Plane's intake: requests are numbered items in a Triage state, statuses read on the day, the Email reading agent files requests
_2026-11-02_

**Context** (plan v4 §4.5, 0.11.8): Plane's intake — requests in the Triage state with a priority, assignees, labels and a due date; Open and Closed tabs; Accept (edit it, choose the state), Decline, Snooze until a date, Mark as duplicate, Delete; filters, sort, up and down through what's pending; the description, activity and comments. Plane's forms and email intake are paid; New Dawn files an email's requests itself.

**Decision — a request is a work item in Triage** (0040 `pm_intake_item`, one row an item): as Plane's, an intake item is a numbered work item of its project — description, priority, assignees, labels, due date, comments, activity — in the project's **Triage** state (made the first time Intake is used, under a free name; it never shows among the project's states to pick), with its intake row saying how it stands. Lists, filters, views, search, the bulk bar and due-date reminders skip Triage items; a Triage item can't be given a state, a cycle, a module or a parent — it is accepted first. So **VIFC-n** is its key from the start, and accepting it moves it into the chosen state (the project's default when none) with its activity row — a work item like any other from then on.

**Decision — statuses read on the day**: pending, accepted, declined, snoozed (until a day after today) or duplicate (of a work item of the project, with a Duplicate of relation). A snooze isn't woken by a job: on its day and after, it reads pending — the list, the count and the actions all ask the day. Closed requests (accepted, declined, duplicate) take no action; one not accepted can be deleted with its comments and files.

**Decision — the Email reading agent files requests** (D-056): when a project has Intake on, the agent's prompt lists it as taking requests and asks for `requests` — work the email asks of such a project, by its exact name, with a short title and a sentence quoted from the email (kept only when the email says it). Each becomes a request in that project, from "Email reading agent", its description the quoted sentence and the email's sender and subject; once a title an email. With no project taking requests, the prompt is as it was. A request stays when its email is forgotten (it is a numbered item).

**Where the owner sees it**: a project's **Intake** tab ("Intake · 3" while three wait) — Open and Closed with their counts, a sort (newest, oldest, priority, due date) and a priority filter, the requests on the left (↑ ↓ or J K go through them), the chosen one on the right: its key and status, its title, the actions while it is open — a state and **Accept**, **Decline**, **Snooze until** a day, **Duplicate of…** a work item found by key or words —, its description, priority, assignees, labels, due date, comments and activity; Delete. **New request** (`N`) asks for a title, a description, a priority, assignees, labels and a due date.

**Means for future work**: Home and Your work (0.11.9) count what waits; analytics (0.11.10) read intake; Plane's API (0.11.11) serves it; the Projects agent (0.11.12) can propose a request's state.

---

## D-080 — Plane's pages: one editor with page blocks, versions as merged snapshots, a lock on the words, PDF through the print sheet
_2026-11-02_

**Context** (plan v4 §4.5, 0.11.7): Plane's project pages — the editor's blocks (text, headings 1–6, quote, callout, divider, numbered, bulleted and to-do lists, table, image, nested page, code with a language picker, Mermaid, emoji, text and background colours, the `/` menu, `@` mentions of work items), an outline, lock, archive, favorites, public or private, full screen, version history and export. Wiki, collections, page sharing, columns, embeds, publishing and live collaboration are Plane's paid or web parts.

**Decision — the one editor grows page blocks** (D-063's TipTap, no new dependency): a page's editor turns on headings 4–6, a **callout** (an emoji and a colour: Info, Note, Warning, Success, Danger), **text and background colours** as marks carrying a colour's name (the stylesheet maps names to tokens, so dark mode and printing follow), a **code block's language** (a select while the caret is in one; Mermaid stays code — no diagram library), a **sub-page** block that opens the page it names (its name read live), and the **`/` menu** of all of them. `@` in a page searches work items, and a click on a mention opens the item in the peek. Emoji come from the Mac's own picker (⌃⌘Space). The item and CRM editors keep their blocks.

**Decision — versions are merged snapshots** (0039 `pm_page_version`): each saved change of the name or the document keeps the page as it then is; the same hand's saves within ten minutes are one version (as an item's description, D-074). Restoring makes a version's name and document the page's again, as a version of its own.

**Decision — a lock on the words, an archive on everything**: a locked page takes no edit of its name, emoji, document or labels, and no restore — its access, star, lock and archive still change. Archiving takes the sub-pages too and unstars them; an archived page is read-only, and only an archived page is deleted (Plane's), with its sub-pages and their files. Private and public are flags (one owner, D-073). A page's pictures live in its own folder, as a work item's.

**Decision — PDF through the print sheet**: Export as PDF opens the Mac's print sheet for the window (`WebviewWindow::print`), where Save as PDF is; the print stylesheet shows the page alone, in the light palette (dark tokens are screen-only). Markdown is written by Rust from the document (headings, marks, lists, to-dos, quotes, callouts, code with its language, tables, pictures, mentions, sub-pages) to the file the save dialog names. The DOCX converters (D-024) stay for minutes, which have a template.

**Where the owner sees it**: a project's **Pages** tab — All · Public · Private · Archived, Search, Sort (last updated, last created, name), Label and Favorites as filters, ★ first; **New page** (`N` or `D`) opens it. A page: its emoji and name (Untitled until named), the editor, Versions, Full screen (Escape leaves), ⋯ Lock, Make private, Duplicate, Export as Markdown, Export as PDF…, Archive (Restore and Delete when archived); beside it the Outline (a heading's line scrolls to it), Labels, the pages inside it (New sub-page) and who made it. A starred page is in the sidebar's Favorites.

**Means for future work**: Power K (0.11.9) finds pages; Plane's API (0.11.11) serves them and their Markdown; the Projects agent (0.11.12) can draft a page through the same versions.

---

## D-079 — Plane's modules: several a work item, Needs attention from dates and blockers, every Projects star a favorite row
_2026-11-02_

**Context** (plan v4 §4.5, 0.11.6): Plane's modules — a project's larger pieces of work: a name unique in the project, a description, a start and a target date (either or both), a status (Backlog, Planned, In progress, Paused, Completed, Cancelled), a lead and members, links; work items in several modules; List, Gallery and Timeline layouts with search, sort and filters; a module's details — progress, the work by state group, what needs attention (overdue, blocked), breakdowns by state, assignee, label and priority, links; favorites; archive when completed or cancelled; delete leaving the items.

**Decision — several a work item** (0038 `pm_module`, `pm_module_item` keyed by the pair, `pm_module_member`, `pm_module_link`): an item's modules are a set — the item's Modules property and the create sheet replace it, a module's Add existing and a bulk change add to it, a group dropped into moves it there from the group it left (Plane's) — each join and leave an activity row ("added it to the module …"), in the same transaction as the rest of the edit. Only the project's modules take work. An archived module is read-only: nothing joins or leaves it (an item keeps it; a copy of the item doesn't join it), its links stay and it takes no star. An item carries its modules by name (`PmItem.modules`), so every layout and the workspace's views show them without another query. A module's name is unique in its project compared as the app compares names (NFC, any case; the index backs it up for ASCII), its control characters dropped (the names are read joined by them). Dates are optional, either alone, the start not after the target (CHECKs as 0037's). The status is the owner's — dates don't make it, unlike a cycle's (D-078) — and archiving takes a completed or cancelled module only, which is then read-only. Deleting a module leaves its items, each with its activity row. Archived items aren't counted.

**Decision — Needs attention is computed**: an open item of the module (not completed, not cancelled, not archived) whose due date has passed on this Mac's day, or that is blocked by an item still open (a Blocking is stored as the other item's Blocked by, D-075). Nothing is stored; marking the blocker Done takes the item off the list. The details read the work by state group (Backlog to Cancelled) and the breakdowns as cycles' do.

**Decision — every Projects star is a favorite row**: the sidebar's Favorites hold a cycle or a module as they hold a view (0036): `favorite.pm_cycle_id`, `favorite.pm_module_id`, each cascading. 0038 copies the cycles starred under 0.11.5 (not archived ones) and `pm_cycle.is_favorite` is read no more; archiving a cycle or a module unstars it (Plane's). A starred cycle or module opens its project's tab on it. A custom object can't take a name whose link column would be one of these (`create_object`).

**Decision — one Gantt**: the Work items Timeline's axis, bars and drag (D-076) became one component (`Gantt.tsx`) that the Modules timeline uses too — a bar dragged moves both dates, an end one, and only the dates a row has are written.

**Where the owner sees it**: a project's **Modules** tab — List · Gallery · Timeline (in the address), Search, Sort (name, progress, start, target, newest), filters (status, lead, member, From–To, Favorites, Archived too), ★ first; **New module** (`N` or `M`) with a name, a description, the status, the dates, a lead and members. A module opens with its ★, its status (a select), its dates, ⋯ Edit and Archive, and Delete; its description, lead and members and progress; its work items (Add existing, New work item, × to take one out); Needs attention (Overdue, Blocked), Progress by state group, Assignees, Labels, Priorities, States, and Links (add, remove). Work items gain **Modules**: in their properties and the create sheet, as a display property, as a grouping (an item in several modules sits in each group; an archived module's group takes nothing), as a filter and in the bulk bar (Add to module). With a project's Modules feature off, none of this shows.

**Means for future work**: analytics (0.11.10) read modules' progress; Plane's API (0.11.11) serves modules and their items; the Projects agent (0.11.12) can propose an item's module.

---

## D-078 — Plane's cycles: dates make the status, one cycle an item, the open work moved on once, a burn-down from completion days
_2026-11-02_

**Context** (plan v4 §4.5, 0.11.5): Plane's cycles — a title, a description, both dates or none (a draft), never overlapping in a project; Draft, Upcoming, Active, Completed; tabs Active / Upcoming / Completed with search and date filters; items added or made in a cycle, one cycle an item; the unfinished work of a completed cycle transferred once; a details panel with the burn-down and breakdowns by assignee, state and label (points when estimates are on); favorites; archive when completed; delete leaving the items; `N` `C`; Active cycles across projects. Start and complete early are Plane's paid tier; ours.

**Decision — the dates make the status** (0037 `pm_cycle`): a cycle has both days or none (a CHECK, with the days as YYYY-MM-DD so their text order is their order, and the start not after the end); its status is read on this Mac's day — Draft without days, Upcoming before its start, Active within, Completed after its end or once completed early. No two cycles of a project overlap (checked in Rust; a cycle completed early frees its last day for the next). Start now moves an upcoming cycle's start to today; Complete now ends an active one today.

**Decision — one cycle an item** (`pm_cycle_item`, the item its key): putting an item in a cycle moves it from its other one, with an activity row ("cycle: Sprint 1 → Sprint 2"); only a cycle of the item's project that isn't completed takes work — from the cycle's Add existing, the item's Cycle property (and the create modal's), a bulk change, a group dropped into, or an item made in it — each in the same transaction as the rest of the edit. A completed cycle's group takes no drop and no quick add; a draft saved in a cycle that has since ended loses it. Deleting a cycle leaves its items in none, each with its activity row. The open work (not completed, not cancelled) of a completed cycle moves on once into a cycle that isn't completed; its counts and its details are kept as they stood (Plane's progress snapshot), so it still reads "2 of 5 done" with its whole burn-down. Archived items aren't counted. The automations (D-075) neither archive nor close what a cycle that hasn't ended holds.

**Decision — the burn-down reads completion days**: no snapshots a day — each day from the start to the end (up to today) counts the cycle's items not completed by that day's end (`completed_at`, set entering a Completed state and cleared leaving it, as this Mac's day), cancelled ones out of scope; the ideal line runs from all the scope to none; points instead of items when the project's active estimate is in points (an item without one weighs nothing). The breakdowns count the items done of all by assignee, state and label.

**Where the owner sees it**: a project's **Cycles** tab — Active · Upcoming · Completed · Draft (with counts), search, a From–To range, Archived too on Completed, ★ first; each row its days and progress; New cycle (`N` or `C`). A cycle opens with Start now or Complete now, Move open work to… (a completed one, once), ⋯ Edit, Favorite, Archive, and Delete; its progress; its work items (Add existing, New work item, × to take one out); the burn-down (inline SVG: dashed ideal, solid remaining) and the breakdowns. Work items gain Cycle as a property, a display property, a grouping (List and Board), a filter and a bulk change. Projects → **Active cycles** lists every open project's running cycle with its days left. With a project's Cycles feature off, none of this shows. A starred cycle is a favorite in the Cycles tab; the sidebar's Favorites take cycles with modules (0038).

**Means for future work**: modules (0.11.6) take the same membership shape, several a work item; analytics (0.11.10) read the same burn-down; the Projects agent (0.11.12) can propose an item's cycle.

---

## D-077 — Plane's filters and views: rules turned into SQL, views as rows a project or the workspace, pinned like records
_2026-11-02_

**Context** (plan v4 §4.4, 0.11.4): Plane's filters — Title, State, State group, Assignees, Priority, Mentions, Labels, Created by, Start, Due, Created and Updated date with their operators (any of, none of; is, before, after, between, "or on", not, empty) — and its views: a project's (any layout) and the workspace's (All, Assigned to me, Created by me, Subscribed, then its own, as a spreadsheet), saved from the current layout, filters and display; updated, duplicated ("Copy of"), deleted, favorited; private and locked.

**Decision — a rule a field, turned into SQL in Rust**: filters are rules `{field, op, values}` — the title (contains), the state, its group, the priority and who made it (any of / none of), assignees, labels, subscribers and the people a comment mentions (any of / none of, "none" for no assignee or no label), the start, due, created and updated days (is, is not, before, after, on or before, on or after, between, empty, not empty). The rules all hold; a rule's values, any one; a rule without values is no rule. Days are YYYY-MM-DD or words — today, tomorrow, yesterday, this, next or last week (from the owner's first day of the week, D-072), this, next or last month — resolved on this Mac's day. `filters.rs` builds the SQL with every value bound, the fields and operators from fixed lists; one query (`items::query`) serves a project's tab and the workspace's views (every open project's items, none archived).

**Decision — views are rows** (0036 `pm_view`): a project's (its project) or the workspace's (none); the rules as one JSON array (nested), the layout and display as `pm_display`'s columns (with the calendar's span and the timeline's zoom; a view doesn't keep folded board columns); private and locked as flags — one owner, so they're labels, not rights; a locked view takes only its unlocking. A person deleted for good leaves its rules (a trigger), merged people become the kept one, once. The sidebar's Favorites hold a view like a record (`favorite.pm_view_id`) and open it — the workspace's views page, or its project's Work items.

**Decision — an open view is a working copy**: opening a project's view puts its filters and display on the Work items tab; changing them changes that copy (the filters in the address, the display in the page) and the bar says so — Save view writes it back (not when locked), Save as a new view makes another, Close view returns to the tab's own remembered display (D-076), which a view never changes. Without a view the tab's filters live in the address, and Save as a view keeps them with the layout and display as they stand.

**Where the owner sees it**: a filter bar under the Work items toolbar (Filters adds a rule; each rule a chip saying what it keeps, "Due date is this week", opened to change it; Clear all) and Save as a view; a project's **Views** tab (★ for Favorites, the layout, the filter count, Private, Locked; ⋯ Rename, Lock, Make it private, Duplicate; Delete; New view); Projects → **Views** for the workspace's — the four built in and the saved ones on the left, the spreadsheet of every open project's work items on the right with the filter bar, Save view and Save as a new view.

**Means for future work**: cycles and modules add their fields to the rules (0.11.5, 0.11.6); the Projects agent (0.11.12) can read through the same query; Plane's API (0.11.11) takes the same rules.

---

## D-076 — Plane's layouts: five views of one list, a drop is one change, the display remembered per project
_2026-11-02_

**Context** (plan v4 §4.4, 0.11.3): Plane's layouts — List, Board (groups and sub-groups, drag between columns), Calendar (by due date, drag to move), Table (columns, inline edit, keyboard navigation, bulk), Timeline (bars by start and due date, drag to change) — and its display options (the properties shown, group, sub-group, order, show sub-items, show empty groups), remembered per user and project.

**Decision — five views of one list**: every layout reads the same items (sub-items as the display says), ordered and grouped by the List's own code (D-074); a layout is how they're drawn. The layout, grouping, sub-grouping and order can come from the address (a link); what it leaves out comes from the project's display, and every change is kept there — 0035 `pm_display`, a row a project (one owner, where Plane keeps one per user), its fields as columns and its two lists (the properties shown, the Board's folded columns) as JSON arrays; a person deleted for good leaves no folded column, and merged people move theirs. The display properties gain Attachment count and Link (0034's rows) and, in the Table, Created on, Created by and Updated on.

**Decision — a move is one change**: an item dropped into another group takes what the group means — a state, a priority, a label or an assignee in place of the one it left (added when it had none of them; taken off when dropped on No label or No assignee). On a Board with sub-groups a card dropped into another column and row takes both at once: one update, one transaction, an activity row a field. The Calendar's drop sets the due date and moves the start with it (the item keeps its length); a Timeline bar moves both dates, its ends one each, never past the other. The drags have keyboard equivalents in the item's own properties, and the Table is the keyboard layout: the arrows move between cells, Enter goes in, Escape comes back out.

**Where the owner sees it**: the Work items tab's toolbar — List · Board · Calendar · Table · Timeline, then Group by, Sub-group by (Board), Order by and Display. The Board: a column a group (folded to a strip and back, remembered), a row a sub-group, + and a quick add in each cell. The Calendar: a month or a week (remembered) from the owner's first day of the week, today marked, the items on their due day, + on a day, and how many have no due date. The Table: a checkbox column with the bulk bar (D-075) and each cell edited where it is. The Timeline: Weeks, Months or Quarters (remembered), today's line, bars and one-day marks, the items without dates below.

**Means for future work**: saved views (0.11.4) keep a layout and a display the same way, named; cycles and modules add Group by Cycle and Module (0.11.5, 0.11.6); the Timeline's dependencies stay Plane's paid tier.

---

## D-075 — Plane's work items, the rest: relations both ways, links and files, bulk checked first, an Inbox fed by others' activity, automations once a day
_2026-11-02_

**Context** (plan v4 §4.3, §4.1, §4.2; 0.11.2): Plane's relations (Relates to, Blocked by and Blocking, Duplicate of), links, attachments (5 MB by default), subscribers and votes; bulk operations in the List; the project automations (auto-archive, auto-close, due-date reminders); the Inbox — notices of what happens on the items you're assigned, made or follow, your mentions and reminders; read and unread, snooze, archive, preferences.

**Decision — relations, links, attachments and votes are Plane's rows** (0034): `pm_relation` keeps one relation a pair of items (a unique index on the pair, either way round); Blocked by is stored once and read from the other item as Blocking; Relates to and Duplicate of read the same both ways; a relation can join items of two projects; the search finds an item by its key ("VIFC-12", "vifc 12", "12") or the words of its title. A link is an address and a title — http, https or mailto (an address without its scheme is taken as https), the same address once an item; it opens in the owner's browser or Mail through the system, never inside New Dawn, which fetches nothing (rule 3). Attachments are files copied into the workspace's `files/pm/<item>/` (beside the description's pictures), up to the owner's limit (5 MB unless changed, 1 to 200); a row keeps the file's name in that folder, never a path. Votes are the owner's own up or down (people don't vote), not on an archived item. Each change is an activity row of the item (of both items for a relation).

**Decision — bulk changes check every item first**: a checkbox a row and a group in the List opens a bar of the properties to change — state, priority, assignees, labels, start, due, estimate — applied by Update, beside Make a copy in…, Subscribe, Archive and Delete. Every item is checked as its own edit would be before anything is written: a refusal names the item and the rule ("VIFC-3: The start date comes before the due date") and nothing changes; Archive refuses the selection while any item isn't completed or cancelled. Then each item is written as it would be alone, with its own activity (ten items' priority, ten rows).

**Decision — the Inbox is fed by the activity**: an activity row written by anyone but the owner — an agent, an automation, later the API — on an item the owner follows makes a notice (a property, a state, completed, a comment); a comment by someone else naming the owner is a mention instead (never both); the automations add due-date reminders. The owner's own doings never reach the Inbox (Plane leaves out whoever acted). Following is the item's subscription: the owner follows what they make, change, comment on or are assigned, and Subscribe stops it. A notice is read or unread, snoozed until a time (1, 3 or 5 days, 1 or 2 weeks, or a date and time) and archived; All and Mentions count their unread, and the sidebar's Inbox shows both. Plane's notification preferences stop only emails; New Dawn mails nothing, so its six kinds (property changes, state changes, work item completed, comments, mentions, due-date reminders) hide a kind from the Inbox and its counts — an app setting (one owner) applied when the Inbox is read, so turning a kind back on shows what it hid.

**Decision — the automations run once a day while New Dawn is open**: 90 seconds after launch and then hourly, the first run of a local day in each workspace with Projects on (the day kept in the workspace's settings, so a missed day is caught up at launch) archives completed or cancelled items nothing happened to (no change, no comment) for the project's months, closes open ones (Backlog, Unstarted, Started) idle for its months into the chosen cancelled state (else the project's first), and adds a reminder for each open item the owner follows that is due today or tomorrow — once a due date. They write as "Automation" through the items' own functions (activity, notices). Plane spares items in an active cycle or module from auto-archive: that rule comes with cycles and modules (0.11.5, 0.11.6).

**Where the owner sees it**: an item's properties gain Relates to, Blocked by, Blocking and Duplicate of (chips that open the item, ×, and + to find one); under its sub-items, Links (an address and an optional title; open, copy, edit, remove) and Attachments (Attach, open, Show in Finder, remove); ▲ ▼ in its header. The List gains a checkbox a row and a group, and the bulk bar at its foot. **Inbox** sits in the sidebar under Projects (and in ⌘K): All · Mentions, Show (all notices, unread, snoozed, archived), Assigned to me · Created by me · Subscribed by me, Mark all as read; a notice opens its item beside the list. Settings → General: Inbox (the six kinds) and Work items (the attachment size limit).

**Means for future work**: the Projects agent (0.11.12) and Plane's API (0.11.11) reach the owner through the same activity rows; cycles and modules add the auto-archive exception and their bulk properties; the Table layout (0.11.3) takes the same bulk bar.

---

## D-074 — Plane's work items: numbered in their project, every change an activity row, drafts beside them
_2026-11-02_

**Context** (plan v4 §4.3, 0.11.1): Plane's work items — the project's identifier and a number (VIFC-12), a title, a rich description, a state, a priority, assignees, labels, start and due dates, an estimate, a parent and sub-items; a peek in three modes and a page of their own; Activity, Comments, History and Transition; comments with replies, mentions and reactions; the description's versions; drafts; archive; the List layout with its grouping, order and display.

**Decision — items are Plane's own rows** (0033, plan §6, not objects of the record engine): `pm_item` is numbered by its project's `last_sequence`, raised in the same transaction as the insert (`UPDATE … RETURNING`), so a number is never given twice, even after a delete; the key "VIFC-12" is composed when read, so a new identifier renames every key. The state is one of the item's own project (a key on `(project_id, state_id)`): a state items use can't be deleted — Settings → States says how many use it. Assignees, labels and subscribers are rows; people are "me" or a person in the CRM, as the project's members are (D-073), and a person merged or deleted for good moves or goes with their assignments, subscriptions, mentions and reactions. Priorities and kinds of activity are enumerations checked in Rust.

**Decision — every change is an activity row**: making an item writes "created" with its state; a change writes one row a field changed (before and after in words, and the states' ids for a state), all in the transaction that writes the change, which is checked whole first; a comment writes "commented". Activity, History (the field changes) and Transition (the time spent in each state) are three readings of that one table. Entering a completed state sets when; leaving it clears it. Sub-items' progress is done of all but the cancelled. Only a completed or cancelled item can be archived ("Only completed or cancelled work items can be archived"); an archived item is read-only until restored. Delete takes the sub-items and the items' files; a project's delete takes its items' folders. A copy keeps the title, description, priority, dates and assignees, and in another project takes its default state without labels, estimate or parent. The description keeps versions: each save is one, merged into the newest when that is under 10 minutes old; restoring one is itself a new version.

**Decision — comments, mentions, reactions, subscribers**: replies go one level deep; a mention is the editor's person mention, kept as a row (`pm_mention`) for the Inbox (0.11.2); reactions are Plane's eight emoji, on the description or a comment, once a person an emoji. The owner follows an item they made or commented on; an assignee follows the item given to them; Subscribe in the header follows or stops.

**Decision — drafts beside the items**: closing the create modal with something written asks "Keep it as a draft?" (Discard · Keep editing · Save draft). A draft is the whole form (`pm_draft.data_json`, its project kept or cleared with the project); Projects → **Drafts** edits, copies, moves and deletes them; making one checks the form again (a person, label, state, estimate or parent gone meanwhile is dropped) and deletes the draft.

**Where the owner sees it**: the project's **Work items** tab is Plane's List layout — grouped by state (or state group, priority, labels, assignees, created by, none) and ordered manually (by dragging) or by created, updated, start, due or priority, both in the URL; **Display** picks the properties a row shows, sub-items and empty groups, remembered per project on this Mac until saved views (0.11.4). Dragging a row reorders it and, onto another state's or priority's group, changes that too. Each group ends with a quick add that takes the group's state, priority, label or assignee; `N` or `I` opens the create modal (project, title, description, every property, Create more). A row opens its **peek** — at the side, as a modal or full screen (the URL keeps it) —, its own page or a window of its own: the header (the key, Subscribe, ⋯ with Copy ID, Copy as prompt, Make a copy, Copy to…, Open in a new window, Open the full page, Archive or Restore, Delete), the title, the description with its reactions and versions, the sub-items with their progress and an add, the activity tabs and the comment box; the properties at the side, with Cycle and Modules shown empty until 0.11.5 and 0.11.6. **Archives** lists the archived items, to read, restore or delete. Pictures in a description or a comment are kept in the workspace's files folder, `files/pm/<id>/`. The update check, the code steps' runner and the menu's commands stay the main window's; an item's window has the app's permissions. The peek, its mode and its activity tab are in the address.

**Means for future work**: 0.11.2 adds relations, links, the attachments list, votes (0034), bulk changes, the automations' nightly run and the Inbox (mentions and subscriptions); the layouts (0.11.3) and saved views (0.11.4) read the same list; cycles and modules fill the sidebar's two empty properties; the Projects agent (0.11.12) proposes items as the CRM agent proposes records.

---

## D-073 — Plane's projects beside the dossier: one project, two sides
_2026-11-02_

**Context** (plan v4 §4.2, 0.11.0): Plane's projects — an identifier that names their work items (VIFC-12), a description, a logo and a cover, a network, a lead and defaults, features (Cycles, Modules, Views, Pages, Intake), states in five fixed groups, labels, estimates, automations — and New Dawn's project dossier (D-054: status, phase, milestone, risks, decisions, the Projects agent's updates), which the owner already uses.

**Decision — one project, two sides**: `project` stays the project (its name, its dossier, its meetings, the agent's updates) and `pm_project` (0032, one row a project, its key the project's) holds Plane's side: identifier, description, emoji, cover, network, lead, default assignee, default subscribers, time zone, the five features and the automations as columns (plan §6's `pm_project_feature` and `pm_automation` tables became columns), the active estimate, a favorite flag, an order, archived. A project made before 0.11.0 — or through the dossier's own path — gets its Plane side the first time the module reads the workspace: an identifier from its name (the initials of two words or more, else the first five letters; accents folded; a number added until unique) and Plane's five default states. A new project is made whole in one transaction. The dossier is the project's **Overview** tab.

**Decision — Plane's rules, kept in Rust**: an identifier is 1 to 10 letters or digits, upper case, unique in the workspace (archived projects keep theirs — their items are named by them). Lead, default assignee and subscribers are "me" (the owner — the CRM's workspace member and the people memory's "owner" are the same person, mapped at the edges) or a person in the CRM; a person already set stays even from Deleted records, a new one must be live; a person deleted for good leaves none behind (a trigger), merged people move. States sit in Backlog, Unstarted, Started, Completed and Cancelled (and Intake's triage, later); each group keeps one state, one state is the default (a unique index), names are unique in the project; the colour, the description and the order within a group change. Labels nest one level (a label group); a group's labels go with it. Estimates are points (numbers) or categories, 1 to 6 distinct values; one system is active, a new one archives it, and the active one's values change in place (a value keeps its place, so an item's estimate stays). Automations: archive closed items after N months, close idle items after N months into a cancelled state (the project's first one when none is chosen), due-date reminders — kept here, run with the work items (0.11.2). Names compare in full (NFC, any case; accents count). States and labels keep a colour **name** of Twenty's palette (green, turquoise, sky, blue, purple, pink, red, orange, yellow, gray), drawn from the app's tokens in light and dark — never a hex value. The settings' edits are checked first and written in one transaction, only the fields given (a merge or a state deleted meanwhile is never written back). **Archive** takes a project out of the list, the Projects agent's reading and Today, and keeps everything; Restore brings it back.

**Where the owner sees it**: **Projects** is Plane's list — cards (a cover band, slate by default or the app's orange: no stock images, nothing leaves the Mac, and the status colours keep their meaning; the logo; the identifier; the dossier's waiting updates) or a list; search, network, order (favorites first, name, newest, identifier), Archived; ★ and ⋯ (Settings, Archive). **New project** is Plane's two steps: cover, logo (an emoji), name, identifier (suggested until changed; a taken one is said at once), description, network, lead — then the features. A project's page: its logo, name and identifier, the dossier's status, and tabs — Overview, Work items, the features on (Cycles, Modules, Views, Pages, Intake), Archives, Settings (General, Members, Features, States, Labels, Estimates, Automations; the section is URL state). The tabs the next releases fill show their columns and ghost rows now.

**Means for future work**: work items (0.11.1) reference `pm_state` (a state items use can't be deleted), `pm_label`, `pm_estimate_point` and use "me" or person ids; the automations run there; Intake (0.11.8) adds its triage state.

---

## D-072 — The CRM agent proposes into the record engine; Today reads the CRM; the owner's formats
_2026-11-02_

**Context** (plan v4 §5.7, §3.7, 0.10.15): New Dawn's experts propose and the owner applies (D-054). With Twenty's CRM rebuilt (D-060 … D-071), the CRM is kept the same way: what a meeting or an email brings in — companies, people, deals, notes, tasks — proposed into the engine's objects. Today (D-057) didn't read the CRM. Twenty's Experience settings set the time zone and how dates, times and numbers read; New Dawn had one fixed format.

**Decision — one CRM agent, the experts' shape**: Workspace → Agents lists it (its guide `agents/crm.md`, its model, its numbers: applied of proposed, per model). After each summary (background lane, once a summary version) and after each email the owner had read (background), one call of the local model reads the text against the CRM records it names (companies; people with their title and company; opportunities with their stage and company; at most 60 an object) and answers one JSON object — companies, people, opportunities, notes, tasks — each with one sentence copied from the text. The app keeps only what holds up: the quote is in the text (letters and digits, case and accents aside), a name, not already in the CRM (names match whole words — "Lan" isn't in "plan"; a person also by first or last name), not the owner, not a person called a company (a name the text writes after Mr., Ms., Mrs., Dr.), a stage the opportunities' select has (else its first), an amount above zero with a three-letter currency (else the field's own), real dates; a person's or an opportunity's company known or proposed beside it; a note or a task about a record known or proposed beside it. Nothing twice: a proposal waiting or dismissed (from any source) isn't made again, nor one already applied from the same meeting or email; the waiting ones a rewritten summary no longer says go. An email's tasks stay the Email reading agent's (D-056): the CRM agent proposes none from an email. A meeting's ⋯ → **Find CRM proposals** reads it now (the owner's lane).

**Decision — proposals are rows of their own, applied through the engine**: `crm_proposal` (0031) keeps the proposal, its quote, its source (a meeting or an email, deleted with it), the record it belongs to (by name, and by id once that exists) and the fields to write in the engine's names. Plan §5.7 calls the proposals records of the engine; they're kept beside it instead, because a proposal isn't CRM data — it must not appear in views, search, exports, the local API or the workflows' triggers. **Apply** writes through the engine as "AGENT · CRM agent" (the timeline and search follow), once: the row is claimed before anything is written, and given back if the write fails. A company, person or opportunity already in the CRM by that name (added by hand since, say) is the one applied — never a second. The record it belongs to is found at Apply — the one named (still live), one made since by that name, or a proposal still waiting from any meeting or email, applied first (a person at a new company applies the company too); a company the owner dismissed leaves a person or an opportunity without one; otherwise Apply says it isn't in the CRM yet. A task already open in Tasks with the same title (the Tasks agent adds the owner's own, D-042; this meeting's first) is attached to the record, not made twice; a task made is the owner's, with its meeting, as Add to Tasks makes it (D-056). Merging two people moves their proposals. Both decisions go to the ledger (D-049).

**Where the owner sees them**: the CRM's **Waiting for you** tab (beside Commitments; ghost cards when none) — each card what it would write (the fields as the tables show them), the record it belongs to, the quote, the AI marker with the model and the meeting or email, Apply and Dismiss. A company's, a person's and an opportunity's page show the proposals about them above their tabs — ghost cards when none (the objects the agent proposes about; a note, a task or a custom object has no such section). **Today** counts the CRM's proposals under Waiting for you and gains **Opportunities to close**: not won (the stages' last option, Twenty's Customer, whatever it is renamed), closing within 30 days (soonest first), then those past their date within 90 days (latest first, in red) — older ones are left to the CRM, since a pipeline without a Lost stage keeps them open for good.

**Decision — the owner's formats, app-wide** (Settings → General → Formats; registry `formats`): a time zone (this Mac's, or any the system knows), the date (24 Oct 2026 · 24/10/2026 · 10/24/2026 · 2026-10-24), the time (24-hour · 12-hour), numbers (1,234.56 · 1.234,56 · 1 234,56 · 1’234.56) and the week's first day (Monday · Sunday · Saturday). Every window formats through one module (`src/kernel/format`): a calendar day (YYYY-MM-DD) stays the day written, a time shows in the zone. A window reads the formats before it draws; when they change (`FormatsChanged`) the main window draws its page again — the shell, its dialogs and its once-a-window work (the update check, the code steps' runner) stay as they are — so a switch changes the tables at once; the Settings window shows each choice in its own format. Today's week, the CRM calendar, the meetings' "this week" and Tasks' "Next week" (next week's Friday, whichever day starts it) start on the chosen day; the calendar and the meetings' groups place a time on its day in the chosen zone; a date typed in the meetings search reads month first when dates are written so; a time typed in a cell is this Mac's; a zone this system doesn't know is read as this Mac's. "Today" itself (due today, overdue) stays this Mac's day — the Rust side has no time-zone table — and headings that spell a day out ("Monday 2 November") keep their words. The language stays English (plan §3.7).

**Means for future work**: the Projects agent on Plane's work items (0.11.12) takes the same rows-beside-the-module shape; a proposal to change an existing record (a title, a stage) fits the same table.

---

## D-066 — Emails and calendar from local sources: Apple Mail answers for a record's addresses, meetings are the calendar
_2026-11-02_

**Context** (plan v4 §3.5, 0.10.9): Twenty connects a mailbox and a calendar and shows, on people, companies and opportunities, the threads whose participants match the record's addresses (a company: anyone at its domain) and the meetings matched by attendees — with a visibility level, contact auto-creation and a blocklist under Settings → Accounts. New Dawn has no sync: Mail on this Mac and the meetings already here.

**Decision — Mail is asked, never synced** (`modules/mailbox.rs`, D-056's way): when a record's Emails tab opens, one AppleScript asks Mail for the inbox messages from the record's addresses (the newest hundred each) and the newest sent messages to any of them; Mail must be running (never launched by New Dawn) and allowed under Automation; the addresses travel as the script's arguments, never inside it; what is read follows the visibility setting (who and when · the subject too · the first words as well). Mail is asked for each property of a whole list in one event — never one event a message — bounded by a year back, the newest hundred an address kept; the inbox rows carry no recipients (that would be an event a message), the sent rows do. The answer is kept — `message_thread` by subject (Re:, Fwd: and their kin stripped) and the other side's domains, a message without a subject a thread of its own; `message` by Message-ID; `message_participant` by address — and shown from there until the next read, ten minutes later or on Refresh. A message opens in Mail by its `message:` URL; Send email is a `mailto:` draft. A received time is Mail's local time (no zone), ordered among its kind only.

**Decision — the links are the addresses** (nothing points at a record): a thread belongs to a record through its participants' addresses — a person's emails, a company's people's emails and its `@domain`, an opportunity's company's — so deleting a record never touches the cache and a new address finds its threads at the next read. A participant links to the person with that address, else to the company owning the domain. People the CRM doesn't know are listed under the threads per Contact auto-creation (nobody · anyone · only those written to), never the owner's own addresses, never a blocked one; Add person makes the record (its company by the domain); Block adds the address or the domain to the blocklist and empties the cache, so what they sent is gone at the next read. The owner's own addresses are Mail's accounts', read with the threads. These settings sit in the Settings window's **Accounts** pane (Twenty's place) although they are the current workspace's — the pane says so.

**Decision — the calendar is the meetings**: a record's Calendar tab lists the summarised meetings whose attendees include the person (a company: any of its people; an opportunity: its company's) — upcoming then past — with the attendees linked to their records and a Recording chip when the audio is kept; it is `crm::calendar`, the dossier's matching, no table of its own (the plan's `calendar_event_target` is not needed).

**Means for future work**: the CRM agent (0.10.15) reads the same threads for a record's briefing; workflows (0.10.11) get a "Send email" step that opens the draft; the local API (0.10.14) serves the threads read-only; Plane's inbox (0.11.2) is a different inbox (notifications), not this one.

---

## D-065 — Import and export: Twenty's spreadsheet import through the engine's own checks, a view's CSV as exactly its columns
_2026-11-02_

**Context** (plan v4 §3.4 rows 79–80, 0.10.8): Twenty imports CSV / XLSX / XLS (10 000 rows a file), maps columns to fields with "Label / Part" headers, links relations by a unique field, updates existing records by an identifier, validates every row with errors fixed in place, and exports a view's visible columns with its filters and sorts (20 000 rows).

**Decision — one path for every object, no second validator**: `kernel/records/transfer.rs` turns a file into the record JSON the engine already takes (a composite's parts merged, a relation resolved to an id) and asks the engine's own value check for each field (`query::check_value`), so what the sheet flags is what a save would refuse; only what a file needs beyond that lives here — the identifier that names an existing record, duplicates in the file and against the records, required fields on new rows, relation look-ups by id, label, unique field, domain or email (remembered per run). Rows with errors are skipped, never half-written; the actor on the records and the timeline is the import, named after the file.

**Decision — CSV by the standard library, workbooks by `calamine`**: `kernel/csv.rs` (RFC 4180) reads and writes; XLSX / XLS / ODS are read by `calamine` (the one Rust dependency the plan lists for this; its add was accepted), the first sheet, serial dates as ISO text. Headers follow Twenty's export ("Phones / Primary Phone Number"), so a Twenty file and a New Dawn file map by themselves; the usual spreadsheet words (email, website, firstName, companyDomain…) map too.

**Decision — the export is the view**: the UI sends the query it lists with (filters, sorts, search) and the visible fields in order; the file is the Id and those columns and nothing hidden, a relation as its label and its id, so the file re-imports as updates.

**Decision — the flows are URL state** (`/crm?tab=…&import=true`, `&export=true`): ⌘K, the ⋯ menu and any link open the sheet or run the export on the page that owns the view; the flag clears at once so Back never repeats it. Runs are kept (`import_job`, `export_job`, migration 0025) and listed under Workspace → CRM → Data model.

**Trust**: the import reads, and the export writes, the path the native dialog gave — from Rust, unscoped, like the templates, files and audio commands before it. That holds for a local single-user app whose webview runs bundled code only (CSP `default-src 'self'`); the commands must never be reached by remote content. Nothing from a file or the UI reaches the SQL text: table and column names come from the metadata (validated identifiers), every cell is bound.

**Means for future work**: the local REST API (0.10.14) imports through the same `transfer::run`; Plane's CSV importer and exports (0.11.10) reuse `csv.rs` and the sheet's steps with Plane's columns; row multi-select → export (§3.2) calls `export_view` with the ids as the filter.

---

## D-064 — Navigation: favorites and the Workspace section as rows, one ⌘K for the whole app, Twenty's keys
_2026-11-02_

**Context** (plan v4 §3.4 rows 76–78, 0.10.7): Twenty's sidebar has Favorites (records, views; folders; personal; dragged to reorder) and a Workspace section listing the objects (reordered in a customise mode, hidden ones shown again from the data model), and a command menu (⌘K) that jumps to any record, view or action, creates records and lists the keyboard shortcuts. Plane's Power K (0.11.9) is the same idea.

**Decision — the sidebar's CRM section is data** (migration 0024): `favorite_folder` and `favorite.folder_id`; `navigation_item` (one row per record object: position, hidden, folder — made at first read for objects without one, so a custom object appears at the end); `recent_visit` (object, record, when — the last hundred, written by the record page; a record deleted for good or an object dropped forgets its visits). `kernel/records/navigation.rs` resolves what the sidebar shows (a pinned record's label through the object's label field, a pinned view's name) and leaves out what is gone. One member, so favorites are the workspace's. The section lives inside New Dawn's sidebar, under the app's own items; the icon rail shows none of it.

**Decision — one ⌘K, built from the metadata**: `CommandMenu` lists Recents, the records that match (`search_records`, with labels), "Create a …" for every record object (then the name, then the page), Go to (the app's pages, every object, every view by name), and the page's actions (edit the record page layout, deleted records of the object, the sidebar, copy the link, the shortcuts) — the page actions reach their page as window events (`newdawn:edit-layout`, `newdawn:show-deleted`), so the menu knows no page's state. ⌘↩ runs the first result. The toolbar's field stays the meetings search; ⌘K is the menu.

**Decision — Twenty's keys where macOS leaves them**: ⌘K, `/` (record search), `g` then `p c o n t` for the objects and `m d` for Meetings and Today (a one-second chord, never while typing or while a sheet is open), ⌘↩, Esc — beside the app's ⌘[ ⌘] ⌃⌘S ⌥⌘S ⌘, — listed in the menu's Keyboard shortcuts.

**Means for future work**: workflows and dashboards (0.10.10–0.10.12) add their sidebar entries and ⌘K actions ("run a manual workflow"); import / export (0.10.8) join the actions; Plane's Power K (0.11.9) adds its sections to the same menu; favorites of pages and projects use `favorite`'s columns for them.

---

## D-063 — Rich text and files: one TipTap editor, documents as JSON, files in the workspace folder as attachment records
_2026-11-01_

**Context** (plan v4 §3.3 rows 67–70, §5.4, 0.10.6): Twenty's notes and tasks have a block editor body (BlockNote), targets on several records, attachments; every record has a Files tab; records are mentioned in text. Plane's pages (0.11.7) need the same editor. Both modules are distributed with New Dawn, so the editor must be MIT: TipTap (`@tiptap/*` v3, the one dependency this track adds, plan §5.8).

**Decision — one editor, the document stored as JSON, its words extracted in Rust**: `src/kernel/editor/Editor.tsx` (StarterKit's headings, lists, quote, code, rule, marks and links; tables; to-do lists; pictures; `@` mentions of records from `search_records`, which now returns labels) saves the ProseMirror document as JSON text into the RICH_TEXT column, debounced and on blur, only when it changed; a body from before (plain text) loads as one paragraph. `kernel/records/rich.rs::plain` turns the JSON into words — a newline per block, `@Label` for a mention — for the search index (`search::text_of`), the agents and a table cell's excerpt; the table never edits a rich text (its editor is on the page and in the side panel). Pictures and pasted images are attachments of the record, shown through the asset protocol (`convertFileSrc`), never base64 in the document. Code inside a document uses the system's monospace (`--font-mono` in tokens.css) — content, not chrome; DM Sans stays the one UI typeface.

**Decision — files live in the workspace folder, as records**: `workspaces/<slug>/files/<uuid7>_<safe name>` (the name cleaned, 200 MB at most), an `attachment` record through the engine with the link to its record and the member as author — so custom objects have files too, Deleted records / restore apply, and a backup of the folder carries the files. Open and Reveal in Finder go through the opener plugin, Save as… through the save dialog and a copy; Delete is the engine's soft delete (the file stays until a delete for good, which the Files widget doesn't offer yet). Dropped files arrive through Tauri's drag-drop event, kept when they land on the widget.

**Decision — notes and tasks are ordinary records**: their pages are the Fields widget with the body editor across the width, a "Related to" tab (the system junction's rows — Twenty's targets, one row per record, editable) and Files; the Tasks object gets a "By status" board seeded once; tasks made outside the engine (a summary's own pass, Add to Tasks, a typed task) write their timeline row themselves, with the meeting they came from.

**Means for future work**: Plane's pages (0.11.7) reuse the editor and `rich::plain`; comments and descriptions of work items too; the files store serves Plane's attachments; mentions of work items join the `@` list; a slash menu and a bubble menu can come without changing the storage.

---

## D-062 — Record pages: Twenty's page layouts as rows, widgets on a 12-column grid, one side panel
_2026-11-01_

**Context** (plan v4 §3.3, 0.10.4): in Twenty a record's page is a layout — tabs of widgets on a grid (fields, related records, timeline, tasks, notes, files, rich text; later charts, emails, calendar, iFrame) — edited in a layout editor, per object; a list opens a record on its page or in a side panel.

**Decision — a layout is three tables** (migration 0023): `page_layout` (one RECORD layout per object; a DASHBOARD kind for 0.10.10), `page_layout_tab` (title, position, default), `page_layout_widget` (type, title, grid x / y / w / h on 12 columns and 56 pt rows, `config_json` genuinely nested: the fields shown, the relation and whether its table edits, the text). Twenty's default page is made in Rust at first read (`layouts::defaults`: Overview with Fields beside Timeline, a Related tab per one-to-many relation, Tasks, Notes, Files; a person also gets New Dawn's Commitments and Meetings), never at open — an object nobody opens costs nothing. Save replaces the whole layout in one transaction (widgets clamped into the grid; the first tab marked default is the default); Reset deletes it and the defaults return.

**Decision — the UI is metadata-driven, like the table**: one `RecordPage` for every object (`/crm/$object/$recordId`; `/crm/people/$id` still works), its header the label identifier edited in place with the table's cell editors, ‹ › through the list's order (`layouts::neighbours`), ⋯ with Delete (a banner with Restore and Delete forever once deleted) and the layout editor. `FieldsWidget` reuses the cell editors as label · value rows; `RelatedWidget` is the `RecordTable` with a rule on the inverse relation (`locked` when "Allow editing" is off; a record added there gets the link back through `defaults`); the timeline reads `timeline_activity` with the field labels resolved in Rust; Tasks and Notes list through `task_target` / `note_target` and add through the engine (`linked::add`: the record, then a target row); the layout editor is the page itself in an edit state — pointer events snapped to the grid, widgets pushed down when they overlap, tabs edited in a bar — and Save sends the whole layout.

**Decision — the side panel is the Overview's Fields widget**: a view's "Open record in" decides whether a label, a card or an event opens the page or the glass side panel (`RecordSidePanel`: the label, the fields, Open, × / Esc); the panel edits through the same hooks, so the list behind it follows at once.

**Means for future work**: dashboards (0.10.10) reuse the three tables with kind DASHBOARD and the same grid; Charts, Emails, Calendar and iFrame widgets are listed (disabled) in the editor until their releases; the data model settings (0.10.5) link to the layout editor from an object; a custom object gets its `<object>_id` on the target tables when it is made, and until then its Tasks, Notes and Files widgets list nothing. The person's legacy `notes` column is now a LONG_TEXT field, so the old person page's Notes textarea lives on in the Fields widget.

---

## D-061 — Views: a saved way of looking at an object, with a working copy the owner saves or resets
_2026-11-01_

**Context** (plan v4 §3.2, 0.10.2): Twenty's lists are views — which fields in what order and width, with which aggregate; filters as rule groups; sorts; a group-by select field; compact; where a record opens; who sees it. Each object has an "All <X>" default. Changes are not saved until Update view.

**Decision — views are rows of the engine's metadata** (migration 0022): `view` (type, key INDEX for the default, open-in, visibility, group field and group order, the kanban and calendar fields for 0.10.3), `view_field` (visible, position, width, footer aggregate), `view_filter_group` (And / Or / Not, nested) with `view_filter` rules (a field, a composite's part, the engine's operator, the value as JSON), `view_sort`, `view_group` (a grouped table's option order and hidden groups). The default view is made at open for every object; a new view copies its columns. `views::query_of` turns a view into the engine's query, so kanban and calendar pages reuse it.

**Decision — the working copy**: the page keeps a working copy of the saved view (`useWorking`); every change in the bar, a header or the footer edits the copy and the table follows at once; **Update view** writes it back (fields, filters, sorts, groups, grouping, in that order), **Save as new view** makes a view and writes it there, **Reset** drops it. On the default view only Save as new view is offered, as in Twenty. Compact, Open record in and Visibility save at once (they are the view's, not the copy's).

**Decision — aggregates are one SELECT**: the footer's and a group's numbers come from `query::aggregate` (count, count empty / not empty / unique, percent empty / not empty, sum, average, min, max) over the same WHERE the list uses; a grouped table runs one list per visible group plus `query::group_counts`. A currency aggregates its amount; the footer shows it in units.

**Means for future work**: kanban and calendar (0.10.3) are view types on the same rows; favorites of views (0.10.7) use `favorite.view_id`; the record page's related-record widgets (0.10.4) are views too. Twenty's select-field group order and "visible groups" are kept exactly; the group list shows "No value" last.

**Added 0.10.3 — kanban and calendar on the same rows**: a KANBAN view's columns are the options of `kanban_field_id` (a select field; `view_group` keeps their order and hidden ones as a grouped table's), each column a list with "field is value", its head an aggregate of `kanban_aggregate_field_id` (a number, amount or rating) — Twenty's expected amount; a drop on another column updates the field through the engine (the timeline records it), a drop within a column sets `position` to the midpoint of its neighbours. A CALENDAR view lists the month's or week's records with two date rules on `calendar_field_id` and moves one by writing the field (a date-time keeps its time of day). "+ Add view" asks for the type and the field, as Twenty does; the engine refuses a column field that isn't a select, a calendar field that isn't a date, an aggregate field that isn't a number (a view made without its field shows what to pick in Options).

---

## D-060 — The record engine: Twenty's data model in SQLite, every object a real table
_2026-11-01_

**Context** (plan v4 §5.2, 0.10.1): the CRM module rebuilds Twenty, whose heart is a metadata-driven data model — standard and custom objects, 20+ field types, relations, views over any of them. Twenty keeps that in Postgres with a schema per workspace; New Dawn keeps one SQLite file per workspace.

**Decision — objects and fields are metadata, records are rows of real tables**: `object_meta` and `field_meta` (migration 0020) describe every object; the standard ones (person, company, opportunity, note, task, and the system objects noteTarget, taskTarget, workspaceMember, attachment, favorite, timelineActivity) are declared in Rust (`kernel/records/standard.rs`) and synced into the metadata at every workspace open; a custom object is a real table made at run time, a custom field a real column (0.10.5). A composite field (full name, emails, phones, links, address, currency, actor) is a few columns side by side, `<column>_<part>`, as Twenty flattens them; a many-to-one relation is `<column>_id`; a one-to-many is its inverse. Every record table has `position`, the `created_by_*` actor (you, an agent, an import, a workflow, the API) and `deleted_at` (soft delete: Deleted records, restore, delete for good).

**Decision — one query layer, one search index, one timeline**: `kernel/records/query.rs` builds every list from Twenty-shaped filter JSON (AND/OR groups, per-type operators incl. relative dates), sorts, search and pages; create / update / delete / restore / destroy validate per type, keep the FTS5 `search_index` in step (no triggers: custom tables come and go) and write `timeline_activity` rows with the diff. Records cross the bridge as JSON (`unknown` in TypeScript), so the UI is metadata-driven: one table, one cell per field type, for standard and custom objects alike.

**Decision — the people memory and Tasks keep their columns**: `person.name`, `title`, `organisation`, `aliases_json` and `task.done_at` stay, because the meeting pipeline reads them; the engine keeps `name` = first + last, `organisation` = the company's name and (through triggers) `task.status` ↔ `done_at`, and the memory's editor writes the split name and the company link. Organisations became companies at the first open; the one workspace member is the owner.

**Means for future work**: views (0.10.2–0.10.3), record pages (0.10.4), the data model editor with run-time DDL (0.10.5) and the local API (0.10.14) all read the same metadata; an agent that proposes a record goes through `query::create` with its own actor. Select colours are Twenty's names on the Majoris palette (green, red, yellow, orange keep their meaning; the rest read neutral).

**Added 0.10.5 — custom objects and fields are run-time DDL, the metadata is the truth** (`kernel/records/ddl.rs`): a custom object is `object_meta` + `CREATE TABLE custom_<name>` with the common columns, a Name field as label identifier (unless skipped, for a junction), the system fields, and `<name>_id` columns on the five target tables (notes, tasks, files, favorites, timeline) with the inverse fields on those system objects — so notes, tasks and the record page work on it at once; a custom field is `field_meta` + `ALTER TABLE … ADD COLUMN` per `types::columns` (the standard tables' types and defaults: `''`, `'[]'`, `'null'`, `REAL`, `INTEGER`, a required select's default). Names come from labels (`camel`: "Project Assignment" → `projectAssignment`, columns `snake`), checked by `valid_ident` and against the reserved common names; the metadata's names never change after creation (labels do). **A column that points at a custom table carries no foreign key**: SQLite can't drop a constraint, and a dangling one makes every write to the child table fail once the custom table is dropped (even a cascade from `view` into `favorite`); the engine already refuses a relation to a missing record, and `delete_object` empties the link rows and the pointing columns itself. **Repair at open** (`ddl::repair`, after `standard::sync`): every custom object's table and link columns, every custom field's columns — created if missing, so a half-made object is finished at the next open and the DDL never needs a transaction. Twenty's rules kept: standard objects and fields deactivate but never delete; a custom one deletes only once deactivated; unique is refused while duplicates exist (deleted records included); a relation is made from either side and shows on both; many-to-many is a junction object made for you (`<This> <Target>`, no Name field, a many-to-one to each side, a one-to-many on each side marked `junction`). A deleted field's column stays (SQLite's DROP COLUMN can't take a constrained column); a field made again with the same name reuses it.

---

## D-059 — Modules per workspace: Projects and CRM, on by default; off hides, pauses and keeps
_2026-11-01_

**Context** (plan v4 §3, §5.1): the owner works for several companies, and not every workspace needs project management or a CRM — and both are about to grow into Twenty's and Plane's full shape (D-058). People becomes the CRM module's first page.

**Decision — a workspace setting, read through the hub**: the `setting` key `modules` (`{"projects": true, "crm": true}`; a missing or unreadable value means both on) lives in the workspace's own database, so the choice travels with the workspace folder and its backups. Workspace → Overview → Modules has the two On | Off rows; the setup wizard's first step has the same, on. Commands `get_modules` / `set_modules`; `modules::switches::enabled` is the one reader.

**Decision — off means hidden and paused, never deleted**: the sidebar item goes and `ModuleGate` sends the module's routes to Meetings; the Projects agent (`projects::propose`) and the commitments pass (`crm::sync`) return at once; the Email reading keeps proposing the owner's tasks but no project updates or commitments; Today blanks that workspace's projects, commitments and their waiting counts. The rows stay, and everything returns when the module is on again. The people memory (Workspace → People) is not part of the CRM module: transcripts and attendees need it either way.

**Decision — the app's own parts are not modules**: Meetings, Live, Tasks, Email, Today and Workspace are always there.

**Means for future work**: every later release of the two tracks checks the switch where it adds an agent, an automation, a Today section or a ⌘K action; the record engine (0.10.1) keeps its tables whatever the switch says.

---

## D-058 — Twenty and Plane rebuilt inside New Dawn, not embedded
_2026-11-01_

**Context**: the owner asked for Twenty (CRM) and Plane (project management) "from GitHub, integrated". Both are server products: Twenty is a NestJS server and worker with Postgres 16 and Redis (4 containers, ≥ 2 GB); Plane is Django and Celery with Postgres 15, Valkey, RabbitMQ, an object store, three React apps and a live server (13 containers, 4–8 GB). This Mac has 16 GB and runs a 9 B model beside them; both are AGPL-3.0 and New Dawn is distributed; their UIs cannot follow D-033.

**Decision**: New Dawn rebuilds **the same modules, features and flows** as the two open-source repositories (Plane: its community edition) in its own stack — Rust + SQLite per workspace, React + CSS modules, DM Sans — written from their documented behaviour, structure and data formats, **never from their code** (no line copied or translated). Differences are only what the stack forces, each marked in `docs/plan.md` §3–4: one owner (members are you and the workspace's people), nothing beyond localhost (Apple Mail and meetings stand in for their cloud connectors; HTTP, webhooks and the API are local only), the macOS theme. Their paid tiers are listed, not built. The first draft's "sync with a running Twenty or Plane" was dropped at the owner's request; CSV and the local API are the bridges. Parity is checked release by release and audited at the end (0.12.0, `docs/parity.md`).

**Means for future work**: when one of their features is unclear, read their docs (and the repository only for names and formats), build what the docs describe, and record any deliberate difference in the parity tables.

---

## D-057 — Today: what needs the owner, across every workspace, on one page
_2026-10-31_

**Context** (plan v3 §5, Cockpit): the owner works for several companies, one workspace each. Tasks, projects, commitments and meetings each had a page per workspace, so answering "what needs me today" meant opening every page of every workspace.

**Decision — one read-only page fed through the hub**: **Today** (first in the sidebar) shows, for every open workspace:
- open tasks due today or earlier (Tasks' own Today list, overdue in red);
- active projects whose next milestone is due within 7 days or is overdue, soonest first;
- the proposals waiting for a decision (project updates, commitments, tasks from emails), with links;
- the commitments the owner confirmed (the newest 8, the rest counted);
- this week's meetings, Monday to Sunday on this Mac.

One command reads each active workspace from its own pool in the hub (rule 1). Nothing is stored, no model is asked, and there is no migration. A workspace that can't be read (it failed to open at launch, or a query failed) is named at the top of the page; the others still show. The current workspace comes first, and the page follows a workspace switch or rename at once.

**Decision — a row opens its page in its own workspace**: a row from another workspace switches to that workspace first, then opens the page, so the page shows that workspace's data. When there are several workspaces, each row names its workspace.

**Where**: Today at the top of the sidebar. Meetings is still the first screen when New Dawn opens.

**Means for future work**: ticking a task or applying a proposal from Today itself; a menu-bar summary. Commitments still have only the date as said (D-055), so they're listed newest first, not by due day.

---

## D-056 — Email: the owner gives New Dawn one email; the Email reading agent proposes, quoting it
_2026-10-30_

**Context** (plan v3 §5, Email): much of what moves a project or binds someone arrives by email, not in meetings. The plan ruled out an inbox (IMAP, network, credentials): the owner picks what New Dawn reads.

**Decision — two ways in, nothing fetched**: **Read the email selected in Mail** asks Apple Mail through Apple Events (`osascript`, like the minutes' email; the first time, macOS asks to allow New Dawn to control Mail) for the selected message's subject, sender, date, Message-ID and text; **Open an .eml file…** reads a saved email. The .eml reader is New Dawn's own (`modules/eml.rs`: headers with encoded words, multipart with the plain part first, base64 and quoted-printable, HTML turned into lines) — no new dependency. An email is stored in the workspace (`email_item`, migration 0018: its text up to 20,000 characters) so it's read once (same Message-ID) and its proposals can quote it; **Forget** deletes it with its waiting proposals (decided ones stay, unlinked, and project updates lose the quote).

**Decision — the Email reading agent** (Workspace → Agents, its own guide and model): one call, in the owner's own lane (they are waiting), with a note first instead of long thinking. It returns the owner's **tasks** (proposals under the email — migration 0019 `email_task` — until the owner presses **Add to Tasks**, which makes the task theirs with the email as its reason and the date said read as a day when it is one, or **Dismiss**), **project updates** for open projects (proposals on the project's page, through the same checks and duplicate rules as a meeting's, D-054) and others' **commitments** (proposed in People, D-055). Every entry must quote one sentence the email really contains; the email is fenced as data, never instructions (an email can try to instruct a model).

**Where**: **Email** in the sidebar: the two buttons, what came of the last reading, and the emails read with their proposed tasks and links to their updates and commitments; People and a project's page say "from the email “…”" on what came from one.

**Review fixes taken** (migration and architecture reviews): the email's tasks are proposals like every expert's (D-054), not tasks written straight into Tasks — an email is written by someone else; an added one is the owner's (`origin` button, as Add to Tasks on an action item), so the Tasks agent's numbers and lessons never count the Email reading agent's work, and decisions on an email's project updates and tasks go to the ledger under the Email reading agent (`inbox`, no lessons yet); the model's answer is read before anything is kept — an answer that can't be read is an error, so the email can be read again (nulls in the answer read as empty); the email is kept once, by its Message-ID or, without one, a hash of sender, date, subject and text, and a second reading is refused; one entry that can't be written is logged and the rest still land; the owner's own promises aren't commitments; Mail is asked only when it's running, with a time limit, each property in its own `try` and the separator taken out of each value (a missing Automation permission is named, other failures shown); an .eml must be a regular file of at most 25 MB, parsed off the async workers, with nesting, boundaries and header lengths bounded, Windows-1252 read and HTML comments, scripts and styles skipped; the commitments and project updates that came from an email say so, with its subject. A second review of the proposals change: the email's date is read without its weekday (a wrong one made the day today); an added proposal whose task was deleted in Tasks no longer shows "In Tasks"; a task added from an email names it in Tasks.

**Means for future work**: "Watch this mailbox" (opt-in, in the background lane) can come once reading one email has proven itself; replies stay the owner's to write.

---

## D-055 — People become dossiers: commitments from others' action items, last contact from attendance
_2026-10-30_

**Context** (plan v3 §5, CRM): the owner works with the same people across many meetings; what they promised and when the owner last met them lived only inside summaries.

**Decision — commitments without a model** (migration 0017 `commitment`, `person.notes`): the summary already says who will do what by when (its action items, D-038). After each summary, every action item whose owner isn't the meeting owner — not linked to a task, not the owner's name or a part of it, not "unassigned"/"everyone" — becomes a **proposed commitment**: who (matched to a person of the people memory by name or another spelling, else kept as said: a person or an organisation), what, the date as said, and the meeting. Once per item per meeting (a re-run summary adds only new ones). The owner **confirms** it (open), then marks it **kept** or **dropped**, or **dismisses** it (not a commitment). Like the minutes, this step is rules over what the summary agent already wrote: no second model call, nothing to wait for. **Look through earlier meetings** does the same for meetings summarised before.

**Decision — people as dossiers**: **People** in the sidebar (with the number of commitments waiting) lists the commitments waiting across everyone, then everyone in the people memory grouped by organisation, each with when the owner last met them (the newest summarised meeting where they attended) and their open commitments. A person's page: waiting, what they owe, the meetings they attended, the owner's notes, kept and dropped. Organisations are the people memory's organisation names, not a table of their own; names and spellings are still managed in Workspace → People.

**Deleting**: a person deleted keeps their commitments by name; a meeting deleted keeps the commitments the owner decided (unlinked, the owner's record, like tasks, D-042) and takes the ones still proposed.

**Review fixes taken** (migration and architecture reviews): the database keeps a decided commitment when its meeting goes (SET NULL) and the trigger deletes only the waiting ones, so a skipped trigger can't lose the owner's record; merging two people moves the commitments and joins the notes; whether an item is the owner's uses the same rules as Tasks (`Who`), and without the owner's name nothing is proposed; items already done aren't proposed; a person is linked only when exactly one matches (the summary's same-person rule — never a family name alone, never one of two namesakes); names corrected in a meeting (naming check, attendee edit, corrections) carry over to its commitments, like its tasks, and edited or corrected action items re-sync them; a re-run summary drops the waiting commitments it no longer has; statuses only move forward (Undo back to waiting or open); confirmed commitments of organisations and unknown names show under Others, dismissed ones with Undo; the owner isn't in their own People list; one transaction per meeting; the cards carry the AI marker (the action items are the summary agent's words). Confirm and Dismiss aren't recorded in the feedback ledger yet: there is no agent whose guide they would teach.

**Means for future work**: the Email expert (0.9.2) adds commitments from emails; the Cockpit (0.9.3) shows open commitments across workspaces. Dates stay as said — a commitment has no calendar date to sort by until an expert resolves it.

---

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
