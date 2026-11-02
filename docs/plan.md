# New Dawn — Plan v4 (phase 3: the Twenty CRM and the Plane project manager, rebuilt inside New Dawn)

_2026-10-31, revised 2026-11-01. Supersedes `plan-v3.md` (kept as history; its queue shipped as 0.7.8–0.9.3, the two speaker releases still wait). Everything carried over stands: Tauri 2 + Rust engine + React cockpit; one SQLite file per workspace; Liquid Glass shell; DM Sans; Majoris palette; **100 % local AI**; bundle id `co.majorisdigital.newdawn`; MacBook Pro M1 Pro 16 GB. Reference docs: `ai-engine.md`, `document-export.md`, `meetily-teardown.md`. Plain language._

_**Revision 2026-11-01.** The first draft built modules "shaped like" Twenty and Plane. This revision builds **the same modules, with the same features and flows, as the two open-source repositories** — Twenty (`twentyhq/twenty`) for the CRM, Plane (`makeplane/plane`, community edition) for project management — coded differently where our stack forces it (Rust + SQLite + React, one owner, nothing beyond localhost, the Majoris/Apple look), with every exception named in §3–4. The Sync release is dropped. From this base you optimise to your own needs._

_This plan is written to be built unattended. Each release in §7–8 is a **spec sheet**; the inventories in §3–4 are its **checklist** — a release is done when every row it names behaves as the original. What the sheet does not say is not built; a question it does not answer is answered by §11 and noted in `LAST_BUILD.md` under "Defaults taken"._

## 1. Where it stands

**Shipped (0.7.8–0.9.3, none installed yet):** jobs in lanes that resume; the feedback ledger and examples; the model check; two channels; "What I learned"; memory search; **Projects** (a dossier per project, the Projects agent proposes updates after each summary); **People** (commitments, last contact, notes); **Email** (one email from Apple Mail or an .eml; its tasks, updates and commitments proposed); **Today** across workspaces.

**Still waiting:** the two speaker releases need sherpa-onnx or FluidAudio built from GitHub, which the unattended run may not do. Nothing in this plan needs code fetched or built from outside; the two reference clones (§2) are a manual step for you.

**What you asked for (2026-10-31 and 11-01):** project management and the CRM as **modules switched per workspace**; the People tab becomes the CRM; **Twenty** and **Plane** are the models — the same modules, full functions and logic flows, coded our way; no sync; a plan detailed enough, with enough releases, that nothing is missed.

## 2. The rule for this phase: same features and flows, our code

- **What "same" means.** Every page, action and setting in Twenty's user guide and in Plane's community edition (what the repository ships; the paid tiers are listed but not built) exists in New Dawn and behaves the same: the same objects and field types, the same views, layouts, filters and display options, the same record pages, cycles, modules, pages, intake, drafts, archives, notifications, dashboards, workflows, import and export, keyboard shortcuts. Names on screen follow theirs (Opportunities, Work items, Cycles, Modules, Intake, Power K…).
- **What changes, and only this:** the stack (Rust + SQLite per workspace instead of NestJS/Django + Postgres; React + CSS modules instead of their UI kits — no Tailwind, no CSS-in-JS, DM Sans not Inter); **one owner** (no sign-in, members, invitations, roles or billing — "members" are you and the people of the workspace, §5.5); **nothing beyond localhost** (their cloud connectors become local equivalents, §5.6); **Apple owns the shell** (system light/dark, native glass, capsules — Twenty's and Plane's theme pickers become "System"). Every such change is marked in §3–4.
- **The marks:** ✔ the same · ≈ adapted (how is said) · ✗ not possible here (why) · ◌ not in the open-source repository (a paid tier) — built only if you ask.
- **References, not sources.** Clone both repositories into `../Ref/twenty` and `../Ref/plane` next to `../Ref/meetily` (manual step; the unattended run must not fetch code). They are read for **structure, data formats, API shapes and behaviour** — model names, CSV columns, REST paths, state lists, keyboard shortcuts — and their docs are read for flows. **No code is copied or translated** (both are AGPL-3.0; New Dawn is distributed): every line here is written from the behaviour.
- **How fidelity is checked.** Each release's "Done when" walks the original's pages and actions for that area in the dev app, side by side with the docs; the last release (0.12.0) is a parity audit of §3–4 row by row, and the two tables are updated with the result.

## 3. Twenty — inventory and what New Dawn does with each

Taken from the repository (`packages/twenty-server/src/modules`: attachment, blocklist, calendar, call-recording, company, connected-account, dashboard, emailing, messaging, note, opportunity, person, task, timeline, workflow, workspace-member; `engine/metadata-modules`: object, field, relation, view (field · filter · filter group · group · sort), page layout (tab · widget), role and permissions, agent · skill · MCP server, command menu item, navigation menu item, logic function, webhook, validation rule, search field, index; `twenty-front/src/modules`: object-record, views, command-menu, spreadsheet-import, record-export, advanced-text-editor, dashboards, workflow, ai, favorites/navigation, settings) and from the 119-page user guide.

### 3.1 Data model

| Twenty | New Dawn | Mark |
|---|---|---|
| Standard objects: People, Companies, Opportunities, Notes, Tasks | the same five, with every standard field (§5.2) | ✔ |
| System objects: Attachments, Favorites, Timeline activities, Views, Workspace members, Workflows (+ versions, runs, automated triggers), Dashboards (+ page layouts, tabs, widgets), API keys, Webhooks, Note/Task targets, Agent chat threads | the same | ✔ |
| Connected accounts, Message channels / threads / participants, Calendar channels / events / participants, Blocklist, Call recordings | **messages** = emails read from Apple Mail; **calendar events** = New Dawn meetings; **call recordings** = the meeting's recording; blocklist = addresses never auto-created | ≈ (§5.6) |
| Custom objects (label, plural, icon, description, activate / deactivate, label identifier, image identifier) | the same, as real SQLite tables created at run time (§5.2) | ✔ |
| Custom fields, 20+ types: Text, Long text, Number, Boolean, Date, Date & time, Select, Multi-select, Currency, Links, Emails, Phones, Address, Rating, Relation, Array, JSON, Domain, Full name, Rich text, Actor, UUID, Position | the same types, same sub-fields (emails: primary + additional; phones: country code; links: primary + secondary with labels; address: street, city, state, country, postcode; currency: amount + code) | ✔ |
| Select / multi-select options with colours and a default; number decimals; currency and country defaults | the same | ✔ |
| Relations: one-to-many / many-to-one, a field that targets several objects (Note → Person, Company, Opportunity), many-to-many through a junction object with the "relation to a junction object" toggle | the same | ✔ |
| Deactivate a field or object (kept), delete, unique fields, required fields (validation rules) | the same | ✔ |
| Soft delete, "Deleted records", restore, permanent delete | the same (`deleted_at`) | ✔ |
| Full-text search across all objects (`/`) | FTS5 over the label identifier and text fields | ✔ |

### 3.2 Views and record lists

| Twenty | New Dawn | Mark |
|---|---|---|
| Table view: columns from fields, show / hide / reorder / resize, inline cell editing, group by a select field with aggregates per group, aggregate row (count, sum, average, min, max, percent empty…) | the same | ✔ |
| Kanban view: by a select field, column aggregate (count · sum · average · min · max of a numeric field), card fields, drag and drop, compact mode, create in a column | the same (native drag and drop) | ✔ |
| Calendar view: by a date field, month / week, drag to move | the same | ✔ |
| Filters: operators per type (text: contains / doesn't contain / empty / not empty; number & currency & rating: is / is not / ≥ / ≤ / empty / not empty; date: is / relative / past / future / today / before / after or equal / empty; select, relation: is / is not / empty; multi-select: contains…; boolean: is; actor, array, JSON, files: contains / empty); several values match any; related-record fields and composite sub-fields (a company's name, an address's city); standard filters AND; **Advanced filter** with OR and rule groups, shown as a chip | the same | ✔ |
| Sorts: ascending / descending, several sorts, from a column header | the same | ✔ |
| Update view · Create view · Save as new view · Reset; default "All X" views; view name, icon, order by drag, favorites, delete | the same | ✔ |
| Visibility: Workspace / Unlisted | kept as a flag (one owner) | ≈ |
| Open a record in the side panel or the full page; compact view | the same | ✔ |
| Multi-select rows → bulk delete, export, run a manual workflow | the same | ✔ |
| "Time in stage" (from the timeline), "expected amount" (kanban sum) | the same | ✔ |

### 3.3 Record pages and activities

| Twenty | New Dawn | Mark |
|---|---|---|
| Record page = tabs of widgets on a grid; per object; add / remove / rename / reorder tabs, default tab; add / drag / resize widgets; widgets: Fields (chosen fields, grouped), Related records (table or kanban, "Allow editing"), Timeline, Tasks, Notes, Files, Charts, Rich text, Emails, Calendar, iFrame | the same; Emails and Calendar fed by Apple Mail and meetings; iFrame only for `localhost` pages | ✔ / ≈ |
| Header: name, icon / avatar, favorite, actions (delete, manual workflows), previous / next record | the same | ✔ |
| Side panel (quick view, Open to the full page) | the same | ✔ |
| Layout editor from ⌘K "Edit record page layout" or Settings → Data model → object → Layout | the same | ✔ |
| Notes: title, rich body (block editor), attached to several records (targets), attachments | the same (TipTap editor, §5.4) | ✔ |
| Tasks: title, body, due date, status TODO · IN PROGRESS · DONE, assignee, targets; a Tasks page with its views (by status kanban) | the same; assignee = you; New Dawn's own Tasks list (0.7.3) becomes a view of this object, keeping Add to Tasks and the meeting agents' proposals | ✔ / ≈ |
| Timeline per record: field changes, notes and tasks created, emails, calendar events, with the actor | the same, plus meetings attended and emails read | ✔ |
| Mentions of records in rich text; attachments on any record (Files) | the same | ✔ |

### 3.4 Navigation, command menu, import and export

| Twenty | New Dawn | Mark |
|---|---|---|
| Sidebar: workspace switcher, search `/`, Settings, Favorites (folders), object shortcuts (reorder, folders, hide), Workflows, collapse | the same, inside New Dawn's sidebar under a **CRM** group (the rail, labels and ⌃⌘S stay) | ✔ |
| Command menu ⌘K: search all objects, create records, go to objects and views, import / export, deleted records, run manual workflows, keyboard shortcuts, edit record page layout | the same; one ⌘K for the whole app (Plane's Power K lives in it too) | ✔ |
| Keyboard shortcuts (the documented set) | the same keys where macOS allows | ✔ |
| Spreadsheet import: CSV / XLSX / XLS (10 000 rows a file), column → field mapping, uniqueness (update existing by id or a unique field), relations by name, validation and error fixing | the same (XLSX through a Rust crate; if that add is refused, CSV first) | ✔ |
| Export a view to CSV; import by API | the same; API = the local REST API (§3.8) | ✔ |

### 3.5 Calendar and emails

| Twenty | New Dawn | Mark |
|---|---|---|
| Connect Google / Microsoft accounts; mailbox sync; email channels (visibility, contact auto-creation); calendar sync; shared-inbox forwarding; limit imported emails | **Apple Mail as the connected account** (Apple Events, local): emails for a record are read on demand by its email addresses (sender or recipient), listed in the Emails widget with their thread, opened in Mail; contact auto-creation becomes a proposal (D-054); meetings are the calendar events, recordings the call recordings | ≈ |
| Send emails from Twenty | a draft in Apple Mail, pre-filled, from a record or a workflow step; sending stays yours | ≈ |
| Book meetings; email campaigns (sending domain, unsubscribes, topics) | no sending infrastructure on a Mac | ✗ |

### 3.6 Workflows, AI, dashboards

| Twenty | New Dawn | Mark |
|---|---|---|
| Triggers: record created / updated / deleted / upserted (with field conditions), manual (one record, several, global), schedule (cron), webhook | the same; schedules run while New Dawn is open and catch up at launch; the webhook trigger is a `localhost` endpoint of the local API | ✔ / ≈ |
| Steps: create · update · find · delete · upsert record, branches (if / else), iterator, delay, form (input mid-workflow), AI agent, send email, HTTP request, code (JavaScript) | the same; send email → Mail draft; HTTP → localhost only; code → JavaScript run in a sandboxed Web Worker of the window (the app must be open); AI → the local model | ✔ / ≈ |
| Versions (draft → active → deactivated), runs with per-step input / output and errors, test run, variables between steps | the same | ✔ |
| Workflow credits | cloud billing | ✗ |
| AI chatbot (ask questions over the CRM, find records), AI agents (name, instructions, model, tools: find / create / update records), agents as workflow steps, agent chat threads on records | the same, on Qwen through `kernel/ai/` (tool calls through Ollama); models = the installed local ones | ✔ / ≈ |
| AI permissions and roles; MCP server for outside tools | one owner (no roles); an MCP server over the local API is optional, late | ✗ / ≈ |
| Dashboards: widgets on a grid (bar, line, pie, aggregate with ratio / prefix / suffix, table with "Allow editing", rich text, iFrame; gauge on their roadmap), data source object + filters + aggregate + group by (select, date buckets, relations), duplicate widget, workspace-level | the same (charts as inline SVG); iFrame localhost-only | ✔ |

### 3.7 Settings

| Twenty | New Dawn | Mark |
|---|---|---|
| Data model: objects (standard, custom, inactive), fields (add with type picker, options with colours, edit, deactivate, delete), relations, layout | the same, under Workspace → CRM → Data model | ✔ |
| Experience: theme (light / dark / system), language, time zone, date format, time format (12 / 24 h), number format, calendar start day — each or "System settings" | time zone and the four formats the same (app-wide, under Settings → General); theme follows macOS by New Dawn's rule; language stays English | ≈ |
| Workspace (name, logo), Profile | exist (Workspace page, Settings → General) | ✔ |
| Developers: API keys, webhooks, playground | API keys and webhooks for the **local** API (§3.8); the playground is the generated docs page of that API | ✔ / ≈ |
| Accounts (emails, calendars) | Apple Mail (§3.5) | ≈ |
| Members, roles, SSO, domains, security, billing, admin panel, lab, applications / marketplace, community, legal | one owner, no cloud | ✗ |

### 3.8 API

| Twenty | New Dawn | Mark |
|---|---|---|
| REST (`/rest/<objects>`, API key, cursor pagination, filters, `PATCH`), GraphQL, auto-generated docs per workspace, webhooks (record events to a URL) | a **local REST API** on `http://localhost:<port>` with Twenty's paths and JSON shapes, API keys, generated docs; webhooks to `localhost` only; no GraphQL | ≈ |

## 4. Plane (community edition) — inventory and what New Dawn does with each

Taken from the repository (`apps/api/plane/db/models`: workspace, project, issue (+ activity, comment, reaction, relation, link, attachment, subscriber, vote, mention, sequence), state, label, estimate, cycle, module, view, page, intake, draft, favorite, sticky, recent_visit, notification, analytic, exporter, importer, api token, webhook, deploy board, asset, session, social connection; `app/views`: the same areas plus search, timezone, external; `apps/`: web, space (published boards), admin (instance), api, live, proxy), the docs index, and the pricing matrix (Free = what the repository ships).

### 4.1 Workspace and shell

| Plane | New Dawn | Mark |
|---|---|---|
| Workspaces (create, switch, settings: name, logo, slug) | New Dawn's workspaces | ✔ |
| **Home**: greeting with weekday / date / time, widgets Quick links, Recents (All / Work items / Pages / Projects), Your stickies; Manage widgets (on / off, reorder) | the same, as the Projects module's first page | ✔ |
| **Your work**: summary (created / assigned / subscribed counts), workload by state group (Backlog · Not started · Working on · Completed · Cancelled), by priority, "work items by state" pie, recent activity; tabs Assigned / Created / Subscribed | the same | ✔ |
| **Inbox** (notifications): every tracked event on work items you're assigned to, created or subscribed to, mentions, due-date reminders; read / unread, snooze, archive; preferences per kind | the same, in-app only (no email, no push); the actors are you, the agents and automations | ✔ / ≈ |
| Drafts page, Archives (work items · cycles · modules), Favorites, Recents, Stickies (colours, formatting), Quick links | the same | ✔ |
| **Power K** (⌘K): search the workspace, actions, navigation | inside New Dawn's ⌘K | ✔ |
| Keyboard shortcuts (the documented set: `N` `I` create, `G` `R` archives, …) | the same | ✔ |
| Customize navigation (show / hide sidebar items) | the same | ✔ |
| Profile settings: theme, time zone, notification preferences, activity, API tokens | the same, theme = system | ✔ / ≈ |
| Members, invitations, roles, guests, SSO, billing, audit logs, governance, instance admin ("God mode"), mobile apps, live collaboration | one owner, one Mac | ✗ |
| Integrations (GitHub, GitLab, Slack, …); importers from Jira, Linear, Asana, ClickUp | cloud | ✗ |
| Publish a project to the web (the `space` app) | no web serving | ✗ |

### 4.2 Projects

| Plane | New Dawn | Mark |
|---|---|---|
| Create: name (unique), identifier (≤ 10 letters / numbers, unique, from the name), description, cover image, logo / emoji, network (Public / Private), lead, features step | the same (network kept as a label) | ✔ |
| Projects list: cards and list, search / filter / sort, favorites, archived; join / leave | the same minus join / leave | ✔ / ✗ |
| Settings → General: cover, logo, name, description, identifier, network, time zone, Archive, Delete | the same | ✔ |
| Settings → Members: members, roles, lead, default assignee, subscribers | lead, default assignee and default subscribers from you + people; no roles | ≈ |
| Settings → Features: Cycles, Modules, Views, Pages, Intake (on / off) | the same | ✔ |
| Settings → States: add, rename, reorder, colour, description, the default, five fixed groups (Backlog, Unstarted, Started, Completed, Cancelled) each with ≥ 1 state | the same | ✔ |
| Settings → Labels: name, colour, label groups (parent) | the same | ✔ |
| Settings → Estimates: Points (Linear / Fibonacci / Squares / Custom) or Categories (T-shirt…), ≤ 6 values, one active system, archive older | the same | ✔ |
| Settings → Automations: auto-archive closed items after N months, auto-close inactive items after N months into a chosen cancelled state, due-date reminders | the same | ✔ |
| Settings → Imports (CSV), Exports | the same | ✔ |
| Time tracking / worklogs, Milestones, Releases, Project updates, Work item types and custom properties, Templates, Workflows and approvals, Recurring work items, Project states, Initiatives, Epics, Teamspaces, Customers, Project overview | paid tiers — not in the repository | ◌ |
| New Dawn's project dossier (0.9.0: phase, next milestone, risks, decisions, the Projects agent's proposals) | kept as the project's **Overview** tab — ours, not Plane's | ≈ |

### 4.3 Work items

| Plane | New Dawn | Mark |
|---|---|---|
| Properties: ID (`PLN-42`), title (≤ 255), description (rich editor, version history), state, priority (urgent · high · medium · low · none), assignees, labels, start / due dates, estimate, cycle, modules, parent, sub-work items (progress), created / updated / completed by and at | the same; assignees = you and the people of the workspace | ✔ / ≈ |
| Create modal: project, title, description, every property, Create more, Save as draft prompt; quick add at the end of a list / group / column / calendar day (inherits the group's value); `N` `I` | the same | ✔ |
| Peek view: side peek (default) · modal · full screen; full page; header: subscribe, copy link, copy as prompt, make a copy, open in new tab, archive, delete, remove from cycle / module; votes | the same (new tab = a new window) | ✔ |
| Sub-work items (create new or add existing, progress bar, filters), relations (Blocked by, Blocking, Relates to, Duplicate of), links (URL + title), attachments (5 MB default), comments (rich text, images, mentions, reactions, edit with "edited", delete, one level of replies), reactions on the description, subscribers | the same | ✔ |
| Activity: All · Activity · Comments · History (before / after) · Transition (time in each state); filters and sort | the same (Updates · Worklogs · Development are paid) | ✔ / ◌ |
| Archive (completed / cancelled only; read-only, searchable), restore, delete (with sub-items and attachments), make a copy (same or another project), copy branch name | the same | ✔ |
| Drafts: prompt on closing a modal with text, Draft a work item, edit, copy, move to project, delete | the same | ✔ |
| Bulk operations (List and Table): select, change state / priority / assignees / dates / labels / cycle / estimate / modules, copy, subscribe, archive, delete, with validation | the same (Plane sells it; it is in our scope) | ✔ |
| Create work items via URL | a `newdawn://` link that opens the create modal pre-filled | ≈ |
| Work item types, custom properties, templates, approvals, customer requests, timeline dependencies, "Implements" relations, intake forms / email | paid tiers | ◌ |

### 4.4 Layouts, display options, filters, views

| Plane | New Dawn | Mark |
|---|---|---|
| Layouts: List (groups, manual order by drag, parent / child shown), Board (groups + sub-groups, drag between columns), Calendar (by due date, drag to move), Table / spreadsheet (columns, inline edit, keyboard navigation, bulk), Timeline / Gantt (bars by start and due, drag to change) | the same (Gantt dependencies and critical path are paid) | ✔ / ◌ |
| Display properties: ID, Assignee, Start / Due date, Labels, Priority, State, Sub-work item count, Attachment count, Link, Estimate, Modules, Cycle (Type, Dependencies, Releases, Milestones, Customers are paid); Table also Created on / by, Updated on | the same | ✔ |
| Group by: State, State groups, Priority, Cycle, Module, Labels, Assignees, Created by, None; Sub-group by the same; Order by: Manual, Last created, Last updated, Start date, Due date, Priority; display filters: show sub-work items, show empty groups | the same | ✔ |
| Filters: Title, State, State group, Assignees, Priority, Mentions, Label, Cycle, Module, Created by, Start / Due / Created / Updated date (is, before, after, between, "or on", not, empty), any-of / not-any-of; Clear all | the same, with every date operator (Plane sells some) | ✔ |
| Views: project views (any layout) and workspace views (built-in All · Assigned to me · Created by me · Subscribed, plus custom; spreadsheet); save, update, duplicate ("Copy of"), delete, favorite; public / private, lock | the same; private and lock as flags | ✔ / ≈ |
| Per-user display settings remembered per project and view | the same | ✔ |

### 4.5 Cycles, modules, pages, intake, analytics, data

| Plane | New Dawn | Mark |
|---|---|---|
| Cycles: title, description, both dates or none (draft), no overlap, states Draft · Upcoming · Active · Completed, tabs Active / Upcoming / Completed, search and date filters, add or create items, one cycle per item, transfer unfinished work (once), details panel with burn-down and breakdowns by assignee / state / label, favorites, archive completed, delete (items stay), `N` `C` | the same; start / complete early included (Plane sells it); auto-schedule and parallel cycles are paid | ✔ / ◌ |
| Modules: title (unique), description, date range, state (Backlog · Planned · In Progress · Paused · Completed · Cancelled), lead, members, layouts List · Gallery · Timeline, search / sort / filters, items in several modules, progress with "Needs attention", links, favorites, archive, delete | the same | ✔ |
| Pages (project pages): editor blocks — text, headings 1–6, quote, callout, divider, numbered / bulleted / to-do lists, table, image, nested page, code with language picker, Mermaid, emoji, text and background colours, `/` menu, `@` mentions of work items; outline; lock; archive; favorites; public / private; full screen; version history; export | the same (Mermaid if its library add is allowed, else a code block); wiki, collections, nested-page sharing, columns, embeds, publishing, collaboration are paid or need the web | ✔ / ◌ |
| Intake (in-app): items in the Triage state with priority, assignees, labels, due date; Open / Closed tabs; Accept (edit, choose the state), Decline, Snooze until a date, Mark as duplicate, Delete; filters and sort; up / down through pending; description, activity, comments | the same (forms and email intake are paid) | ✔ / ◌ |
| Analytics: Overview (projects, work items, active cycles, intake), Work items (by state, Created vs Resolved, Customized insights by priority / assignee / state / labels / cycles) | the same; Projects, Cycles, Modules and Intake tabs are paid — a light version of each is ours | ✔ / ◌ |
| Exports: work items as CSV / XLSX / JSON with the current filters, export history | the same | ✔ |
| CSV importer: Plane's columns (Name, Description, Priority, Start Date, Due Date, State Name, Labels, Assignees, Subscribers, Comments, Links, Cycle Name, Estimate, Identifier, Parent), header matching, value formats, matching rules, migrations table and JSON summary | the same | ✔ |
| Webhooks, API tokens, REST API (`/api/v1/workspaces/{slug}/projects/…`, `X-API-Key`), MCP server | the local API with Plane's paths; webhooks to localhost; MCP optional | ≈ |
| Plane AI (Pi chat, agents, skills, memory) | New Dawn's own agents on the local model (§5.7) | ≈ |

## 5. How it is built

### 5.1 Modules
Projects and CRM are modules switched per workspace (workspace setting `modules`, on by default; Workspace → Overview → Modules and the setup wizard). Off = sidebar section gone, routes open Meetings, agents and automations paused, Today and ⌘K skip it; data kept. The people memory (Workspace → People) stays with CRM off — transcripts need it. Meetings, Live, Email, Today and Workspace are the app.

### 5.2 The record engine — Twenty's metadata model in SQLite (`kernel/records/`)
- **Metadata tables** (migration 0020): `object_meta` (name singular / plural, labels, icon, description, is_custom, is_active, label identifier field, image identifier field, position) and `field_meta` (object, name, label, type, options JSON — select options with colour and position, number decimals, currency default, relation target(s), kind and junction —, default value, is_custom, is_active, is_nullable, is_unique, position, description, icon). The standard objects and their fields are declared in Rust (as Twenty's workspace entities are) and **synced into the metadata at every open**; what you add is custom.
- **Every object is a real table.** Standard tables come from migrations (0021: `company`, `opportunity`, `note`, `note_target`, `task_target`, `attachment`, `timeline_activity`, `favorite`, `workspace_member`; `person` and `task` exist and gain Twenty's columns). A custom object or field is **DDL at run time** through the hub (`CREATE TABLE custom_<name>`, `ALTER TABLE … ADD COLUMN`), recorded in the metadata; deactivating keeps the column; a repair step at open recreates what the metadata says and the table lacks. Composite types (full name, emails, phones, links, address, currency, actor) are JSON text columns read with `json_extract`; many-to-one = `<field>_id` + index; many-to-many = a junction object. Common columns on every object: `id`, `created_at`, `updated_at`, `deleted_at`, `position`, `created_by` (actor JSON: you · agent · import · workflow · API).
- **Query layer:** one builder turns a view (filters with their operators and groups, sorts, group by, aggregates, search, pagination) into SQL for any object; `records_list / get / create / update / delete / restore / destroy / batch`, `objects_* / fields_*`, `search`. A global `search_index` (FTS5: object, record, text) is maintained on write. Timeline activities are written on every change with the field diff.
- **UI:** metadata-driven components — `RecordTable`, `RecordBoard`, `RecordCalendar`, `RecordShowPage` (page layouts), `RecordSidePanel`, and one input per field type — so a custom object gets every view and page for free, as in Twenty.

### 5.3 The project manager — Plane's models, our schema
Plane's community edition is not metadata-driven, so the Projects module keeps a fixed schema that mirrors Plane's models one to one (§6): project (+ features, automations, estimate system), state, label, estimate point, issue (+ assignee, label, link, attachment, comment, comment reaction, reaction, vote, activity, subscriber, relation, mention, sequence), cycle (+ cycle issue), module (+ issue, link, member), issue view (project / workspace), page (+ label, version), intake (+ intake issue), draft issue, sticky, quick link, recent visit, favorite, notification (+ preference), user display properties, exporter / importer history. The existing `project`, `project_meeting`, `project_update` tables stay for the Overview tab. Layouts, display options and filters are Plane's own components, not the record engine's views (their logic differs; both are built faithfully).

### 5.4 Shared parts (built once, used by both)
Rich text editor (TipTap, MIT — the block set of §4.5, mentions of records and work items, images from local files; stored as editor JSON + plain text for search and the agents); files and attachments (`workspaces/<slug>/files/`, size limit setting, previews); favorites and recents (one sidebar section, both modules); notifications inbox; the single ⌘K (Twenty's command menu + Power K); keyboard shortcut registry; formats (time zone, date, time, number, week start) in Settings → General; the local REST API (one server, two path families, API keys, webhooks to localhost); CSV / XLSX read and write.

### 5.5 One owner
Where the originals have members, New Dawn has **you and the people of the workspace** (the people memory / CRM People): assignees, leads, members, subscribers, point of contact, account owner. Actors on records and activities are you, an agent (by name), an automation, an import or the API. No sign-in, roles, invitations, sharing or billing anywhere; "private / public / unlisted" survive as plain flags.

### 5.6 Local only
Twenty's Google / Microsoft sync → Apple Mail through Apple Events (read on demand per record, drafts for sending) and New Dawn's meetings as calendar events and call recordings. HTTP steps, webhooks, iFrames and the API are `localhost` only (rule 3 unchanged; `check-localhost-only.sh` still passes because addresses are data you type, refused by the engine when not local). AI = the installed local models through `kernel/ai/`. Nothing is pulled from GitHub by the app.

### 5.7 New Dawn's own layer stays on top
Experts propose, you apply (D-054): after each summary and each email the CRM agent proposes people, companies, opportunities, notes and tasks; the Projects agent proposes work items, state changes, comments and the Overview's updates; the proposals are records of the engine / rows of the module, shown where Twenty and Plane show the data. Today keeps reading across workspaces. The names memory, vocabulary, ledger, examples, model check: unchanged.

### 5.8 Dependencies this plan adds (each with its fallback if the unattended add is refused)
npm: `@tiptap/*` (+ `lowlight` for code; `mermaid` optional) — fallback: a plain-text editor with Markdown rendering, reported. Rust: an XLSX reader (`calamine`), a small HTTP server (`axum` on the tokio already there) for the local API — fallbacks: CSV only; the API release waits. Everything else is written here: cron parsing, CSV, charts (inline SVG), drag and drop (native), grids (CSS).

## 6. Data model additions (append-only migrations; each reviewed by `migration-safety`)

| Migration | Tables / columns | Release |
|---|---|---|
| — | workspace setting `modules` | 0.10.0 |
| 0020 | `object_meta`, `field_meta`, `search_index` (FTS5), `timeline_activity` | 0.10.1 |
| 0021 | `company`, `opportunity`, `note`, `note_target`, `task_target`, `attachment`, `favorite`, `workspace_member`; `person` += Twenty's columns (name as full name, emails, phones, links, city, avatar, position, deleted_at, created_by); `task` += body, status, assignee, position, deleted_at, created_by | 0.10.1 |
| 0022 | `view`, `view_field`, `view_filter`, `view_filter_group`, `view_sort`, `view_group` | 0.10.2 |
| 0023 | `page_layout`, `page_layout_tab`, `page_layout_widget` | 0.10.4 |
| 0024 | `favorite_folder`, `navigation_item`, `recent_visit` | 0.10.7 |
| 0025 | `import_job`, `export_job` | 0.10.8 |
| 0026 | `message_thread`, `message`, `message_participant`, `calendar_event_target`, `blocklist` | 0.10.9 |
| 0027 | `dashboard`, `dashboard_widget` | 0.10.10 |
| 0028 | `workflow`, `workflow_version`, `workflow_run`, `workflow_automated_trigger` | 0.10.11 |
| 0029 | `agent`, `agent_chat_thread`, `agent_chat_message` | 0.10.13 |
| 0030 | `api_key`, `webhook`, `webhook_delivery` | 0.10.14 |
| 0031 | `crm_proposal` (the CRM agent's proposals, D-072) | 0.10.15 |
| 0032 | `pm_project` (Plane's columns — features and automations as columns — linked 1:1 to `project`), `pm_state`, `pm_label`, `pm_estimate`, `pm_estimate_point` | 0.11.0 |
| 0033 | `pm_item` (numbered per project: `pm_project.last_sequence`), `pm_item_assignee`, `pm_item_label`, `pm_item_subscriber`, `pm_comment`, `pm_reaction`, `pm_activity`, `pm_item_version`, `pm_mention`, `pm_draft` | 0.11.1 |
| 0034 | `pm_relation`, `pm_link`, `pm_attachment`, `pm_vote`, `notification` (preferences are an app setting, D-075) | 0.11.2 |
| 0035 | `pm_display` (a project's layout and display options, one owner; D-076) | 0.11.3 |
| 0036 | `pm_view` (a project's or the workspace's; rules as JSON), `favorite.pm_view_id` (D-077) | 0.11.4 |
| 0037 | `pm_cycle` (with a progress snapshot), `pm_cycle_item` (one cycle an item; D-078) | 0.11.5 |
| 0038 | `pm_module`, `pm_module_item` (several modules an item), `pm_module_member`, `pm_module_link`; `favorite.pm_cycle_id` / `pm_module_id` (cycle stars copied; D-079) | 0.11.6 |
| 0039 | `page`, `page_label`, `page_version` | 0.11.7 |
| 0040 | `intake`, `intake_issue` | 0.11.8 |
| 0041 | `sticky`, `quick_link`, `home_widget`, `navigation_preference` | 0.11.9 |
| 0042 | `analytic_view`, `exporter_history`, `importer_history` | 0.11.10 |
| 0043 | `api_token`, `pm_webhook` | 0.11.11 |

Conventions as before (UUID v7 ids, `created_at` / `updated_at`, enumerations checked in Rust, BEFORE triggers for meeting deletes). Custom objects and fields are run-time DDL (§5.2), not migrations. Registry: nothing.

## 7. Releases — Track A, the CRM (Twenty), 0.10.0 → 0.10.15

Numbering: this phase starts at **0.10.0**, patch bumps after; the Projects track is **0.11.x** and can be built after, before, or alternately with this one — say which. Each release is published before the next and never installed here (D-044).

**Checks, every release:** `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `pnpm tsc --noEmit`; the screens in the dev app (`NEWDAWN_DEV_OPEN=…`), light and dark and Reduce Transparency; the module off and on again; the original's docs pages for the area walked side by side (the rows of §3 / §4 the sheet names); `migration-safety` per migration (dev app stopped first); `architect-reviewer`; `/build-check`; `LAST_BUILD.md`, `DECISIONS.md`, §6 and §9 updated; version bump, tag, signed build, `scripts/release-local.sh`, `latest.json` verified.

### 0.10.0 — Modules (1 sitting)
- **Goal:** the two switches; People → CRM; nothing else changes yet.
- **Same as:** —.
- **Data:** the `modules` setting; `modules.rs` through the hub; `get_modules`, `set_modules`.
- **Screens and flows:** Workspace → Overview → Modules (two switch rows, hint "Off hides the module and pauses its agents and automations. Its data stays."); the wizard's first step; sidebar sections **CRM** and **Projects** shown when on; `/crm` with the People list (today's page), `/people*` redirects; Projects as today.
- **Module off:** §5.1, enforced in `projects::follow`, `crm::follow`, the Email reading, Today, badges.
- **Done when:** the round trip in both modules in the dev workspace; a Rust test for the default and the round trip.

### 0.10.1 — Record engine and standard objects (4 sittings)
- **Goal:** Twenty's data model in SQLite, the five standard objects with every standard field, and a first table of each.
- **Same as:** §3.1 (standard objects, field types, soft delete, search).
- **Data:** 0020, 0021 (§6). People migrate in place: `name` → full name (first / last split at the last space, aliases kept), `organisation` → the Company relation (companies created from the distinct names), `email` / `phone` / `linkedin` / `city` → Emails / Phones / Links / City; `notes` → a Note. Tasks (0.7.3) become the Task object with status mapped (done → DONE, else TODO), the meeting link as a task target to the meeting? — no: meetings are not records; the link stays in `task.reason` and the Timeline.
- **Screens and flows:** CRM → People · Companies · Opportunities · Tasks · Notes (the sidebar's object shortcuts, §3.4) each a `RecordTable`: columns from the fields, inline editing for every type (the inputs of §5.2), add a record (a new row), delete (soft) and Deleted records with restore and permanent delete, record count, search `/` across objects (results list → record). Twenty's standard fields on each object: People — name, emails, phones, job title, city, LinkedIn, X, avatar, company, point of contact for, created by, position; Companies — name, domain, address, employees, LinkedIn, X, annual recurring revenue, ideal customer profile, tagline, work policy, visa sponsorship, account owner, created by, position; Opportunities — name, amount, close date, stage (New · Screening · Meeting · Proposal · Customer, colours as Twenty's), probability, point of contact, company, owner; Tasks — title, body, due at, status, assignee; Notes — title, body.
- **Agent:** the CRM agent (`Agent::Crm`) proposes people and companies from summaries as records of the engine (D-054).
- **Done when:** every object lists and edits inline in the dev workspace; the migrated VIFC people show first / last names, companies and emails; a deleted person appears in Deleted records and restores; search finds a company by a word of its tagline; the standard field list matches Twenty's entities field for field.
- **Not in it:** views, record pages, rich text (bodies are plain until 0.10.6).

### 0.10.2 — Table views: filters, sorts, groups, saved views (3 sittings)
- **Goal:** Twenty's table view and view bar.
- **Same as:** §3.2 (table, filters, sorts, view settings).
- **Data:** 0022.
- **Screens and flows:** the view dropdown (All X default, create, rename, icon, reorder by drag, favorite, delete, Workspace / Unlisted flag); the view bar (Filter with every operator of §3.2 incl. related-record fields and composite sub-fields, Advanced filter with OR and rule groups shown as a chip, Sort with several sorts, Options → Fields show / hide / reorder, Group by a select field with per-group aggregates and collapse, aggregate row with the operations, compact, open records in side panel / full page), Update view · Create view · Save as new view · Reset; column resize; row multi-select → delete / export (CSV from 0.10.8).
- **Done when:** "People whose company's name contains Keppel OR city is Hanoi, sorted by last name" saves as a view and survives relaunch; the aggregate row sums Opportunities' amount per stage group.

### 0.10.3 — Kanban and calendar views (2 sittings)
- **Same as:** §3.2 (kanban, calendar).
- **Screens and flows:** Kanban by a select field (stages from the field's options; column aggregate count · sum · average · min · max of a numeric field; card fields; drag and drop; compact; create in a column); Calendar by a date field (month / week; drag to move; create on a day); both as view types of the dropdown; "expected amount" = the kanban sum.
- **Done when:** the Opportunities pipeline shows the amount per stage; dragging a card changes the stage and writes a timeline activity; Tasks by due date on the calendar.

### 0.10.4 — Record pages, page layouts, side panel (4 sittings)
- **Same as:** §3.3 (record pages, header, side panel, layout editor).
- **Data:** 0023.
- **Screens and flows:** the record page (header with name / avatar / favorite / actions / previous-next; tabs of widgets on a grid — Fields, Related records (table or kanban, Allow editing), Timeline, Tasks, Notes, Files, Rich text; Charts, Emails, Calendar, iFrame arrive with their releases and the layout editor already lists them); per-object default layouts as Twenty's (Overview with Fields + Timeline; Tasks; Notes; Files); the layout editor (⌘K "Edit record page layout", Settings → Data model → object → Layout; add / drag / resize / configure widgets; add / remove / rename / reorder tabs; default tab); the side panel from any list with Open.
- **Done when:** a company page shows its people in an editable related table; a resized Fields widget persists; the side panel edits a field without leaving the table.

### 0.10.5 — Data model settings: custom objects, fields, relations (3 sittings)
- **Same as:** §3.1 (custom objects / fields / relations / options), §3.7 (Data model).
- **Data:** run-time DDL (§5.2); tests for create / rename / deactivate / delete and the repair step.
- **Screens and flows:** Workspace → CRM → Data model: objects list (standard, custom, inactive; create: labels, plural, icon, description); object page: fields (add with the type picker and per-type options — select options with colours and default, decimals, currency, country code, relation target(s) and kind, junction toggle —, edit, deactivate, delete, unique, required, label / image identifier), relations shown on both sides, Layout; the new object appears in the sidebar with its All view and record page.
- **Done when:** a custom object "Partnership" with a relation to Companies and a many-to-many to People through a junction object lists, filters, kanbans by its own select field and has a record page, with no code change.

### 0.10.6 — Rich text, Notes and Tasks, files (3 sittings)
- **Same as:** §3.3 (notes, tasks, mentions, attachments), §5.4 (editor).
- **Screens and flows:** the TipTap editor (blocks of §4.5 that Twenty's BlockNote also has: headings, lists, to-dos, quotes, code, tables, images, mentions of records with `@`, links); Note and Task bodies; Notes and Tasks created from a record page with targets (several records), the Tasks page with its views (table by due, kanban by status), the Notes page; Files widget with drop-to-upload, preview, download, reveal in Finder, delete; attachments on notes and tasks; New Dawn's Tasks list (0.7.3) becomes the Task object's "My tasks" view and keeps Add to Tasks, the due badge and Today's "Due today".
- **Done when:** a note written on a person with a table and an image shows on the person's and the company's Notes tab; a task from a summary's action item is a Task record with the meeting in its timeline.

### 0.10.7 — Sidebar, favorites, ⌘K, keyboard shortcuts (2 sittings)
- **Same as:** §3.4 (sidebar, command menu, shortcuts).
- **Data:** 0024.
- **Screens and flows:** the CRM section of the sidebar as Twenty's (object shortcuts with reorder, folders and hide; Favorites with folders for views and records; Workflows and Dashboards entries when they exist); ⌘K: search all objects, create records, go to objects / views / settings, import / export, deleted records, run manual workflows, keyboard shortcuts, edit record page layout; Twenty's shortcut set (`/` search, `⌘K`, `g` then a letter, `⌘Enter`…) adapted where macOS reserves keys.
- **Done when:** ⌘K creates a company by name in two keystrokes; a favorited view sits in a folder; the shortcut list matches Twenty's page.

### 0.10.8 — Import and export (3 sittings)
- **Same as:** §3.4 (spreadsheet import, export).
- **Data:** 0025; `kernel/csv.rs` (standard library), XLSX through `calamine` (fallback CSV).
- **Screens and flows:** Import from ⌘K or an object's menu: upload (CSV / XLSX / XLS, ≤ 10 000 rows), map columns to fields (auto-matched by header, per-type parsing, relation by name or id), uniqueness (update existing by id or a unique field), validate with errors you fix in place or skip, import with a report; Export a view (CSV) with the view's filters and columns; import / export jobs listed.
- **Done when:** a Twenty-exported People CSV imports with companies linked; re-importing with changed titles updates instead of duplicating; a view exports exactly its columns.

### 0.10.9 — Emails and calendar from local sources (3 sittings)
- **Same as:** §3.5 (adapted).
- **Data:** 0026.
- **Screens and flows:** Settings → Accounts shows Apple Mail as the connected account (Automation permission on first use); a record's Emails widget lists the threads where any of its email addresses is a sender or recipient, read on demand from Mail (cached as `message_thread` / `message` rows, no bodies kept unless you open them), each opens in Mail; "contact auto-creation" becomes CRM-agent proposals from unknown addresses, with the blocklist; the Calendar widget lists the record's meetings (attendees' records and their companies) with a link to the meeting; the call recording on the meeting; Send email = a pre-filled draft in Mail.
- **Done when:** a person with two addresses shows their Mail threads, newest first; a meeting with Keppel Land attendees shows on the company's Calendar widget.

### 0.10.10 — Dashboards (3 sittings)
- **Same as:** §3.6 (dashboards).
- **Data:** 0027.
- **Screens and flows:** Dashboards in the sidebar; a dashboard = widgets on a grid (drag, resize, duplicate); widget types bar (horizontal / vertical, ≤ 100 bars, group by ≤ 50), line, pie, aggregate (count / sum / average / min / max, ratio for select fields, prefix / suffix), table (a view, Allow editing), rich text, iFrame (localhost); data: object, filters, aggregate, group by (select, date by day / week / month / year, relation); charts as inline SVG in the Majoris palette; the Charts widget on record pages (related records).
- **Done when:** "Pipeline amount by stage" and "Opportunities closed by month" render and update when a card moves.

### 0.10.11 — Workflows I: builder, triggers, record steps, versions, runs (4 sittings)
- **Same as:** §3.6 (triggers, record steps, branches, iterator, delay, form, versions, runs).
- **Data:** 0028; the engine in `modules/workflows/` run by the jobs scheduler (0.8.0), durable state per run.
- **Screens and flows:** Workflows list; the builder (trigger: record created / updated / deleted / upserted with field conditions, manual (record / records / global — appears in record actions and ⌘K), schedule (cron, while the app runs, catch-up at launch), webhook (shown, active with 0.10.14)); steps create / update / find / delete / upsert record, branches, iterator, delay, form (a sheet asking you, the run waits); variables `{{…}}` with a picker; versions (draft, activate, deactivate; editing an active one makes a draft); test run; Runs page with status and per-step input / output / error.
- **Done when:** "when an Opportunity's stage becomes Customer, create a Task 'Send welcome note' due in 3 days on its point of contact" runs on a stage change and its run shows each step; a delayed step survives a quit.

### 0.10.12 — Workflows II: AI, email, HTTP, code (2 sittings)
- **Same as:** §3.6 (the remaining steps, adapted).
- **Screens and flows:** AI agent step (prompt + record context on the local model, output as variables), send email (a Mail draft to a record's address, with subject / body templates), HTTP request (localhost only, refused otherwise), code (JavaScript in a sandboxed Web Worker; inputs / outputs JSON; runs when the window is open, else the run waits and says so).
- **Done when:** the four steps run inside one workflow; a non-local URL is refused with a plain message.

### 0.10.13 — AI chatbot and agents (3 sittings)
- **Same as:** §3.6 (chatbot, agents, chat threads).
- **Data:** 0029.
- **Screens and flows:** "Ask AI" in ⌘K and a chat panel (glass, right side) with threads; the default agent answers over the CRM with tools find records (filters), count / aggregate, open a record, create / update (shown as a proposal card to apply); custom agents under Workspace → CRM → AI (name, description, instructions, model from the installed ones, tools allowed); agents usable as workflow steps; chat threads attached to records (targets).
- **Done when:** "which opportunities above 50k have no activity for 30 days?" lists them with links; "add a note to Keppel Land: …" shows a proposal, Apply writes it.

### 0.10.14 — Developers: API keys, local REST API, webhooks (3 sittings)
- **Same as:** §3.8, §3.7 (Developers).
- **Data:** 0030; `axum` on localhost (a free port, shown in settings), off by default.
- **Screens and flows:** Workspace → CRM → Developers: API keys (name, expiry, revoke, shown once), webhooks (URL — localhost only —, events, secret, deliveries log), the generated docs page; endpoints `/rest/<objects>` list / get / create / update / delete with Twenty's query parameters (filter, orderBy, limit, starting_after) and JSON shapes, `/rest/batch/<objects>`; the workflow webhook trigger endpoint.
- **Done when:** `curl -H "Authorization: Bearer …" http://localhost:<port>/rest/people?limit=2` returns Twenty-shaped JSON; a webhook delivery reaches a local test listener on a record update.

### 0.10.15 — The CRM agent on the new model, Today, Experience formats (2 sittings)
- **Same as:** §5.7 (ours), §3.7 (Experience formats).
- **Screens and flows:** the CRM agent proposes people (with company and title), companies, opportunities (stage from the conversation), notes and tasks from each summary and each email, each quoting, into the engine's objects; proposal cards on the record pages and a "Waiting for you" section on the CRM home; Today: Waiting counts, Tasks due (Task object), "Opportunities to close" (close date within 30 days); Settings → General → Formats (time zone, date, time 12 / 24 h, number, week start — used everywhere dates and numbers are shown).
- **Done when:** the dev meeting proposes an opportunity in Meeting stage with its quote; Apply creates it; Today lists it when its close date is near; the date format switch changes the tables at once.

Track A ≈ **45 sittings**.

## 8. Releases — Track B, project management (Plane), 0.11.0 → 0.11.12, then 0.12.0

### 0.11.0 — Projects and their settings (3 sittings)
- **Same as:** §4.2.
- **Data:** 0032; existing `project` rows get a `pm_project` with an identifier from the name.
- **Screens and flows:** Projects list (cards / list, search, sort, filter, favorites, Archived); create modal (name, identifier, description, cover, logo / emoji, network label, lead, then the features step); project sidebar (Work items, Cycles, Modules, Views, Pages, Intake when on; Settings; Archives; the Overview tab for New Dawn's dossier); Settings: General, Members (lead, default assignee, default subscribers from you + people), Features (five switches), States (groups, add / rename / reorder / colour / description / default), Labels (groups), Estimates (points / categories, ≤ 6 values, one active, archive), Automations (auto-archive N months, auto-close N months into a cancelled state, due-date reminders), Archive / Delete.
- **Done when:** a project created with cycles and intake on shows those sidebar items; a state can't be the last of its group; `VIFC` is refused as a duplicate identifier.

### 0.11.1 — Work items I: model, create, list, peek, comments, drafts, archives (5 sittings)
- **Same as:** §4.3 (properties, create modal, quick add, peek modes, header actions, sub-items, comments, activity, archive / restore / delete, drafts), List layout of §4.4.
- **Data:** 0033.
- **Screens and flows:** the create modal (every property, Create more, Save as draft prompt, `N` `I`); List layout with group by / order by (incl. manual drag) / display properties / display filters; quick add per group; peek (side / modal / full) and full page: title, description (editor + version history), meta row and properties sidebar (state, priority, assignees, dates, labels, estimate, parent, cycle, modules), sub-work items with progress, comments (editor, images, mentions, reactions, edit / delete, one-level replies), activity tabs All · Activity · Comments · History · Transition, subscribe, copy link, copy as prompt, make a copy, open in new window, archive (completed / cancelled only), restore, delete (with sub-items), Drafts page (edit, copy, move to project, delete), Archives page.
- **Done when:** `VIFC-1` is created from a draft, gets two sub-items whose progress shows 50 % after one is Done, a comment with a reaction, and the Transition tab shows its time in In Progress; archiving an In Progress item is refused.

### 0.11.2 — Work items II: relations, links, attachments, subscribers, bulk, automations, inbox (3 sittings)
- **Same as:** §4.3 (relations, links, attachments, subscribers, votes, bulk operations), §4.1 (Inbox), §4.2 (Automations).
- **Data:** 0034.
- **Screens and flows:** relations (Blocked by, Blocking, Relates to, Duplicate of, shown both ways), links (URL + title), attachments (limit setting), subscribers, votes; bulk operations in List (select, change state / priority / assignees / dates / labels / cycle / estimate / modules with validation, copy to a project, subscribe, archive, delete); the automations run nightly in the background lane (auto-archive, auto-close, due-date reminders); the Inbox: notifications for every tracked event by an agent or automation, mentions, reminders; read / unread, snooze, archive; preferences per kind under Settings.
- **Done when:** a due-date reminder lands in the Inbox the day before; auto-close moves a stale Backlog item to Cancelled after the set months (clock moved in a test); bulk-changing ten items' priority writes ten activities.

### 0.11.3 — Layouts: board, calendar, table, timeline (4 sittings)
- **Same as:** §4.4 (layouts, display options).
- **Data:** 0035 (`pm_display`).
- **Screens and flows:** Board (group + sub-group, drag between columns and sub-groups, collapse groups, quick add per column), Calendar (month / week, drag between days, quick add on a day, items without due dates hidden), Table (columns incl. Created on / by, Updated on, inline edit, keyboard navigation, bulk), Timeline (bars by start and due, drag to change dates, zoom); display properties, group / sub-group / order by, display filters per layout; per-project display settings remembered.
- **Done when:** the board sub-grouped by assignee moves an item to another assignee and state in one drop; the timeline shows a dragged bar's new dates in the item.

### 0.11.4 — Filters and views (2 sittings)
- **Same as:** §4.4 (filters, views).
- **Data:** 0036 (`pm_view`, `favorite.pm_view_id`).
- **Screens and flows:** the filter bar with every field and operator; Views page per project (create from the current layout + filters + display options, update, duplicate, delete, favorite, private / lock flags); workspace Views (All · Assigned to me · Created by me · Subscribed, custom; spreadsheet) across projects.
- **Done when:** "Assigned to me, due this week, not Done" is a favorited workspace view that opens from the sidebar.

### 0.11.5 — Cycles (3 sittings)
- **Same as:** §4.5 (cycles).
- **Data:** 0037 (`pm_cycle`, `pm_cycle_item`).
- **Screens and flows:** Cycles page with Active / Upcoming / Completed tabs, search and date filters; create (`N` `C`; both dates or none; no overlap); Draft · Upcoming · Active · Completed; start / complete early; add existing or create items; the Cycle property; transfer unfinished work (once); details panel: burn-down (after start), breakdowns by assignee / state / label, points when estimates are on; favorites; archive completed; delete (items stay); "Active cycles" page across projects.
- **Done when:** a two-week cycle with five items shows the burn-down after two Done; transfer moves the three open items to the next cycle and is then disabled.

### 0.11.6 — Modules (2 sittings)
- **Same as:** §4.5 (modules).
- **Data:** 0038 (`pm_module`, `pm_module_item`, `pm_module_member`, `pm_module_link`; Projects stars as favorite rows).
- **Screens and flows:** Modules page with List / Gallery / Timeline, search, sort, filters (favorites, status, lead, members, dates); create (title unique, description, dates, state, lead, members); items in several modules; details: progress, breakdown by state group, Needs attention (overdue, blocked), breakdown by state / assignee / label / priority, links; favorites; archive (completed / cancelled); delete.
- **Done when:** a module with three items, one overdue and one blocked, lists both under Needs attention; the timeline drags its dates.

### 0.11.7 — Pages (3 sittings)
- **Same as:** §4.5 (pages).
- **Data:** 0039.
- **Screens and flows:** Pages list (search, sort, filters, favorites, archived); the editor with the block set (text, headings, quote, callout with icon and colour, divider, lists and to-dos, table, image, nested page, code with language picker, Mermaid if allowed, emoji, colours, `/` menu, `@` mentions of work items), outline, lock, public / private flag, full screen, version history (local snapshots on save), archive, export (Markdown / PDF through the existing converters).
- **Done when:** a page with a table, a code block and a mention opens the mentioned item from the mention; a locked page refuses edits; a restored version replaces the text.

### 0.11.8 — Intake (2 sittings)
- **Same as:** §4.5 (intake).
- **Data:** 0040.
- **Screens and flows:** Intake page with Open / Closed tabs, filters and sort, up / down through pending; create an intake item (title, description, priority, assignees, labels, due date; Triage state); Accept (edit, choose the state), Decline, Snooze until, Mark as duplicate (links, declines, labels), Delete; the Email reading agent files an email's request as an intake item when the project has Intake on (ours).
- **Done when:** an accepted item becomes `VIFC-n` in the chosen state; a snoozed one returns on its date.

### 0.11.9 — Home, Your work, Power K, stickies, navigation, profile (3 sittings)
- **Same as:** §4.1 (Home, Your work, Power K, shortcuts, customize navigation, profile settings, favorites, recents, stickies, quick links).
- **Data:** 0041.
- **Screens and flows:** Home (greeting, Quick links, Recents with its filter, Stickies with colours and formatting, Manage widgets); Your work (summary, by state group, by priority, pie, recent activity, Assigned / Created / Subscribed tabs); Power K actions inside ⌘K (create work item, go to project / cycle / module / page, toggle sidebar, shortcuts…); the shortcut set; Customize navigation; Settings → Notifications preferences; the Projects section of the sidebar as Plane's (Home, Your work, Inbox, Drafts, Projects with their sub-items, Views, Archives, Favorites).
- **Done when:** the sidebar matches Plane's with Inbox showing its unread count; `G` `R` opens Archives.

### 0.11.10 — Analytics, exports, CSV importer (3 sittings)
- **Same as:** §4.5 (analytics, exports, importer).
- **Data:** 0042.
- **Screens and flows:** Analytics: Overview (projects, work items, active cycles, intake), Work items (by state, Created vs Resolved over time, Customized insights by priority / assignee / state / labels / cycles), light Cycles / Modules tabs (ours), per project and per cycle / module; Exports (CSV / XLSX / JSON with current filters, history); Workspace → Imports: Plane's CSV importer with its columns, matching rules, migrations table and JSON summary.
- **Done when:** a Plane CSV export re-imports with states, labels, parents and comments; Created vs Resolved shows the dev project's week.

### 0.11.11 — Developers: API tokens, local REST API, webhooks (2 sittings)
- **Same as:** §4.5 (API), on the server of 0.10.14.
- **Data:** 0043.
- **Screens and flows:** personal API tokens; `/api/v1/workspaces/{slug}/projects/`, `…/issues/`, `/states/`, `/labels/`, `/cycles/`, `/modules/`, `/intake-issues/` with Plane's JSON and `X-API-Key`, a 60-a-minute limit with the headers; webhooks (localhost) with deliveries.
- **Done when:** `curl -H "X-API-Key: …" http://localhost:<port>/api/v1/workspaces/vifc/projects/` lists the projects as Plane would.

### 0.11.12 — The Projects agent on Plane's model, Today (2 sittings)
- **Same as:** §5.7 (ours).
- **Screens and flows:** after each summary and email, for each followed project: proposed work items (title, assignee, dates, priority, labels, parent, cycle = the active one), state changes, comments quoting the meeting, Overview updates as today; proposal cards in the item and the project; Today: your items due (Assigned to me), cycles ending within 7 days, intake pending, Inbox unread, proposals waiting.
- **Done when:** the dev meeting's action items become proposed items in the active cycle; "the report was sent" proposes Done with its quote.

### 0.12.0 — Parity audit (2 sittings)
- Walk §3 and §4 row by row against the running app and the originals' docs; fix what is missing or different; update the two tables with the result (✔ confirmed, ≈ with its exact difference, ✗ / ◌ unchanged); write `docs/parity.md` as the living record; this closes the phase.

Track B ≈ **39 sittings**. Both tracks: **≈ 85 sittings** — at the 0.7.x pace (two or three a week) about eight months; faster if the unattended run keeps going nightly as it did for phase 2.

## 9. Decisions this plan proposes (recorded in `DECISIONS.md` when each is built)

| # | Decision |
|---|---|
| D-058 | **Twenty and Plane rebuilt, not embedded**: the same modules, features and flows as the open-source repositories, written in our stack from their behaviour; no code copied (AGPL); the repositories cloned by the owner as read-only references (`../Ref/`) |
| D-059 | **Modules per workspace**: Projects and CRM, a workspace setting, on by default; off = hidden, agents and automations paused, Today and ⌘K skip, data kept (0.10.0) |
| D-060 | **The record engine**: Twenty's object / field metadata in SQLite, every object a real table, custom objects and fields as run-time DDL through the hub, one query layer, metadata-driven UI (0.10.1–0.10.5) |
| D-061 | **One owner**: members are you and the workspace's people; actors are you, agents, automations, imports, the API; roles, sharing and billing do not exist |
| D-062 | **Local equivalents**: Apple Mail as the connected account (on demand, drafts to send), meetings as calendar events and call recordings, localhost-only HTTP / webhooks / iFrames / API, local models for AI (0.10.9, 0.10.12, 0.10.14) |
| D-063 | **The project manager keeps Plane's fixed schema** and its own layouts, display options and filters; New Dawn's dossier stays as the Overview tab (0.11.x) |
| D-064 | **Shared parts**: one rich text editor (TipTap), one files store, one favorites / recents, one ⌘K, one inbox, one formats setting, one local API server (0.10.6, 0.10.7, 0.11.2, 0.10.14) |
| D-065 | **Experts stay on top**: the CRM and Projects agents propose into the rebuilt modules; nothing is written without Apply (0.10.15, 0.11.12) |
| D-066 | **Parity is a deliverable**: §3–4 are the checklist, `docs/parity.md` the record, 0.12.0 the audit |

## 10. Not in this plan

- **Everything marked ✗** in §3–4: members, roles, invitations, SSO, billing, admin panels, domains, email campaigns, booking, Google / Microsoft / Slack / GitHub connectors, cloud importers, publishing to the web, live collaboration, mobile apps, workflow credits — they need users, servers or the cloud that New Dawn does not have.
- **Everything marked ◌**: Plane's paid tiers (work item types and custom properties, time tracking, milestones, releases, project updates and states, initiatives, epics, teamspaces, templates, workflows and approvals, recurring items, intake forms and email, customers, wiki and nested pages, dashboards, project overview) — not in the repository; say if you want any and it joins the queue.
- **Theme and language pickers** — macOS decides the theme; the UI stays English (New Dawn's rules).
- **A GraphQL API** — the REST one carries the same data.
- **Speakers** — still waits for your permission to build the runtime (plan v3 §4 stands).
- **Sync with a running Twenty or Plane** — removed at your request; CSV and the local API are the bridges.

## 11. Defaults the unattended run takes (strike any)

1. Numbering 0.10.x for the CRM track, 0.11.x for Projects, 0.12.0 the audit; the CRM track first unless you say otherwise.
2. Both modules on for every workspace; new ones on unless switched off in the wizard.
3. Names on screen are the originals' (Opportunities, Work items, Cycles, Modules, Intake, Inbox, Your work, Power K inside ⌘K).
4. Standard objects and fields exactly as Twenty's entities; stages and colours as Twenty's defaults; Plane's state groups, default states, priorities, module states, estimate systems as documented.
5. People migrate: name split at the last space; organisation → company; existing notes → a Note; Tasks (0.7.3) → the Task object with "My tasks" as the default view and Add to Tasks unchanged.
6. Assignees, leads, members, subscribers = you + the workspace's people; "created by" = you unless an agent, an automation, an import or the API did it.
7. Rich text = TipTap, stored as JSON + plain text; images stored as files under the workspace; the attachment limit 5 MB, changeable.
8. The agents run in the background lane after each summary and each email, one model call each; at most ten proposals a run; nothing applied without you.
9. Schedules and automations run while New Dawn is open and catch up at launch (missed runs are run once, logged).
10. The local API is off until you create a key or a token; it listens on `127.0.0.1` on a free port shown in settings; webhooks, HTTP steps and iFrames refuse any non-local address.
11. No new dependency beyond §5.8; if an add is refused, the fallback named there is built and the difference reported.
12. Drag and drop is native HTML with a menu alternative for the keyboard; charts are inline SVG; grids are CSS.
13. Keyboard shortcuts follow the originals' documented lists, with macOS-reserved keys remapped and listed in ⌘K → Keyboard shortcuts.
14. Each release's "Done when" is checked in the dev workspace against the original's docs for the area, and the matching rows of §3–4 are ticked in `docs/parity.md`.

## 12. Open with you

1. **Track order:** CRM first (as written), Projects first, or alternately one release each?
2. **Clone the references:** `git clone` Twenty and Plane into `../Ref/twenty` and `../Ref/plane` before the run starts (read-only; the run must not fetch them).
3. **Paid-tier features** you want anyway (e.g. time tracking, milestones, project updates, work item types)? They are small once the base exists; name them and they join Track B.
4. **The theme picker:** keep macOS-only (New Dawn's rule), or replicate Light / Dark / System inside the app?
5. Unchanged: the speaker releases wait for your OK to build the runtime; the VIFC restore waits for your new workspace.
