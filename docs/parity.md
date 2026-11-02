# Parity — New Dawn against Twenty and Plane

The living record of plan v4's rule (docs/plan.md §2): the same features and flows as Twenty (CRM) and Plane (project management, community edition), in New Dawn's own code. This file holds each feature row, checked against what is built, and says where it differs.

Audited for 0.12.0 on 2026-11-03, row by row, against the built code and DECISIONS.md (D-058 to D-085). Neither original's source is on this Mac (plan §1: read for behaviour, never copied), so what Twenty and Plane do comes from the plan's rows, their documentation and the decisions.

**Marks**
- ✔ the same
- ≈ ours, with a difference (stated in the row)
- ✗ not built (as the plan chose, unless said)
- ◌ later

Update a row when a release changes it, and keep plan §3–§4's marks in step.

## Summary

Of 99 feature rows:
- **26** are the same;
- **3** are the same with a part kept for later or not built;
- **60** are ours with a difference;
- **8** aren't built;
- **2** are for later.

Almost every ✗ and ◌ is a choice the plan made. Those are members, roles, SSO and billing (one owner), integrations, publishing to the web, campaigns, workflow credits, paid tiers and an MCP server. Two are gaps:
- Twenty's Time in stage on the kanban;
- Plane's Create work items via a URL (the local API's POST is the way in).

The differences come in kinds:
- **Local instead of cloud:**
  - Apple Mail stands in for connected accounts;
  - meetings stand in for the calendar;
  - the local REST API and webhooks reach this Mac only;
  - the local models stand in for Twenty's and Plane's AI.
- **One owner instead of members:** assignees and leads are you and the people the workspace knows; private and locked are labels.
- **Smaller pieces left out**, each named in its row.

## Fixed in 0.12.0 from this audit
- CRM:
  - a Phones field's default country now fills a new number;
  - the iFrame widget is offered on record pages, not only on dashboards;
  - **Run workflow** on a table's selected records (Single workflows once per record, Bulk ones once).
- Projects:
  - automations leave alone items in a module still under way, as they do items in a cycle not ended;
  - a work item's ⋯ has **Remove from <module>**;
  - activity can be read newest first;
  - a project's archived work items can be searched by key or title.

## Known differences worth a later look
- **CRM:**
  - Time in stage on the kanban;
  - unique fields made of parts compare each text part on its own (a unique amount refuses a second one in the same currency). A whole-value check was tried in 0.12.0 and set aside: it touched the engine's write path.
  - exporting a table's selection;
  - actor names on the timeline (an agent's, a workflow's or the API's change shows as "New Dawn");
  - a related record's field in a filter;
  - Duplicate widget;
  - a Search row in the sidebar;
  - `g s` for Settings;
  - a run's step inputs;
  - createdAt and updatedAt as fields.
- **Projects:**
  - Favorites shown when the CRM is off;
  - Power K creating more than work items;
  - Create via a URL;
  - Intake's assignee and label filters;
  - export across projects from workspace views;
  - a Display panel for workspace views;
  - light Projects and Intake tabs in Analytics;
  - the Plane API's comments, links and activity;
  - the assistant (D-070) reading Projects data.

## Twenty (CRM) — plan §3

### §3.1–3.3 Data model, views, record pages
| § | Feature | Plan | Verified | Evidence | Difference or gap |
|---|---|---|---|---|---|
| 3.1 | Standard objects: People, Companies, Opportunities, Notes, Tasks | ✔ | ≈ | D-060, D-071; kernel/records/standard.rs | A task's due date is a date (not a date and time); createdAt / updatedAt aren't fields (no column, sort or filter on them). |
| 3.1 | System objects | ✔ | ≈ | D-060–D-071; standard.rs | All exist; nine are engine objects, the rest (views, versions, triggers, layouts, keys, webhooks, threads) plain tables. |
| 3.1 | Connected accounts, messages, calendar, blocklist, recordings | ≈ | ≈ | D-066; mailbox.rs | As planned: Mail read on demand; the calendar is meetings; the blocklist also drops blocked senders' cached messages. |
| 3.1 | Custom objects and their settings | ✔ | ≈ | D-060; kernel/records/ddl.rs | Every setting kept; the icon and image identifier are stored, not drawn. |
| 3.1 | Field types and sub-fields | ✔ | ≈ | D-060; ddl.rs, crm/cells.tsx | Every sub-field stored; editors change the primary email, phone and link only; JSON and Full name fields aren't created by hand. |
| 3.1 | Option colours, defaults, decimals, currency, country | ✔ | ≈ | DataModelField.tsx, cells.tsx | Multi-select has no default; Address has no default country; the Phones default country fills a new number from 0.12.0 (the calling code isn't derived from it). |
| 3.1 | Relations, multi-target fields, junction toggle | ✔ | ≈ | D-060; crm/record/widgets.tsx | Multi-target = the fixed note, task and file targets; the junction toggle is kept, Related widgets still list junction rows. |
| 3.1 | Deactivate, delete, unique, required | ✔ | ≈ | D-060; kernel/records/query.rs | Unique on a composite checks each text part on its own (a unique amount refuses a second one in the same currency; a unique name, a shared first name); numbers aren't checked on save. |
| 3.1 | Soft delete, Deleted records, restore, destroy | ✔ | ✔ | D-060; query.rs | — |
| 3.1 | Full-text search across objects (`/`) | ✔ | ✔ | D-060, D-064; records/search.rs | — |
| 3.2 | Table view: columns, editing, groups, aggregates | ✔ | ✔ | D-061; crm/RecordTable.tsx | — |
| 3.2 | Kanban view | ✔ | ≈ | D-061; crm/views/KanbanBoard.tsx | A column loads 200 cards at most, no "load more". |
| 3.2 | Calendar view | ✔ | ✔ | D-061; crm/views/CalendarView.tsx | — |
| 3.2 | Filters: operators, sub-fields, advanced groups | ✔ | ≈ | D-061; useViewState.ts, query.rs | A relation rule picks records; no rule on a related record's field. |
| 3.2 | Sorts | ✔ | ✔ | D-061; RecordTable.tsx | — |
| 3.2 | Update / create / save as / reset; manage views | ✔ | ≈ | D-061, D-064; ViewBar.tsx | A view's icon is stored, not set or shown. |
| 3.2 | Visibility: Workspace / Unlisted | ≈ | ≈ | D-061; ViewBar.tsx | As planned: a flag on the view. |
| 3.2 | Side panel or full page; compact | ✔ | ✔ | D-062; ViewBar.tsx | — |
| 3.2 | Multi-select rows → bulk actions | ✔ | ≈ | D-065, D-068; RecordTable.tsx | Delete (restore / destroy in Deleted) and, from 0.12.0, Run workflow on the selection; a selection isn't exported (a view exports as a whole). |
| 3.2 | Time in stage; expected amount | ✔ | ✗ / ✔ | D-061; KanbanBoard.tsx | Time in stage not built; expected amount = the column's Sum. |
| 3.3 | Record page: tabs of widgets on a grid | ✔ / ≈ | ≈ | D-062, D-066, D-067; LayoutEditor.tsx | Related records as a table only; Fields widget without groups; Rich text widget as text; iFrame on record pages from 0.12.0. |
| 3.3 | Header: name, avatar, favorite, actions, prev / next | ✔ | ≈ | D-062, D-068; RecordPage.tsx | No avatar; ‹ › follow the object's order, not the open view's. |
| 3.3 | Side panel with Open | ✔ | ≈ | D-062; RecordSidePanel.tsx | The quick view shows the default tab's fields. |
| 3.3 | Layout editor from ⌘K and Settings | ✔ | ✔ | D-064; CommandMenu.tsx, DataModelObject.tsx | — |
| 3.3 | Notes: title, rich body, targets, attachments | ✔ | ≈ | D-063; kernel/editor/Editor.tsx | The "/" menu is on for Plane's pages, not CRM notes (a toolbar and @ mentions there). |
| 3.3 | Tasks object and Tasks page | ✔ / ≈ | ≈ | D-063, D-042; TasksPage.tsx | Due as a date; the Tasks page is its own list on the same table. |
| 3.3 | Timeline per record with its actor | ✔ | ≈ | D-060; records/timeline.rs, layouts.rs | The record's own changes with diffs; linked notes, emails and meetings not on it; changes by an agent, a workflow or the API show as "New Dawn". |
| 3.3 | Record mentions; Files on any record | ✔ | ✔ | D-063; Editor.tsx, records/linked.rs | — |

Taken in 0.12.0: iFrame on record pages; the Phones default country; Run workflow on a table's selection. Left: exporting a selection; actor names on the timeline. The unique check on a composite's whole value was tried in 0.12.0 and set aside: it touches the engine's write path (a worker panic in the test).

### §3.4–3.8 Navigation, mail and calendar, workflows and AI, settings, API
| § | Feature | Plan | Verified | Evidence | Difference or gap |
|---|---|---|---|---|---|
| 3.4 | Sidebar: switcher, search, favorites, objects, collapse | ✔ | ≈ | D-064; kernel/shell/Sidebar.tsx | No Search row (search is `/` and ⌘K); objects and favorites reorder with ✎ then ▲▼, not by drag. |
| 3.4 | ⌘K command menu | ✔ | ✔ | D-064; CommandMenu.tsx | — |
| 3.4 | Keyboard shortcuts | ✔ | ≈ | D-064; useShortcuts.ts | ⌘K, `/`, g + p·c·o·n·t·m·d, ⌘↩, Esc; no `g s`; no arrow keys between table cells. |
| 3.4 | Spreadsheet import CSV / XLSX / XLS | ✔ | ≈ | D-065; crm/transfer/ImportSheet.tsx | First sheet only, row 1 the header (no step to pick); the Validate step shows 500 rows at most. |
| 3.4 | Export a view to CSV; import by API | ✔ | ✔ | D-065, D-071; api/rest.rs | — |
| 3.5 | Connected accounts → Apple Mail; meetings as calendar | ≈ | ≈ | D-066; mailbox.rs | Inbox (from the addresses) and Sent (to them) only; other mailboxes and Cc on received mail missed. |
| 3.5 | Send email → a Mail draft | ≈ | ≈ | D-066, D-069; crm/record/widgets.tsx | From a record: a person's Emails tab, `mailto:` with To; the workflow step fills To, Cc, Subject, Body. |
| 3.5 | Book meetings; email campaigns | ✗ | ✗ | — | Not built, as planned. |
| 3.6 | Workflow triggers | ✔ / ≈ | ✔ / ≈ | D-068; crm/RecordTable.tsx | Schedules run while New Dawn is open; several records from a table's selection since 0.12.0. |
| 3.6 | Workflow steps | ✔ / ≈ | ✔ / ≈ | D-068, D-069; workflows/run.rs | — |
| 3.6 | Versions, runs, test run, variables | ✔ | ≈ | D-068; workflows/RunPage.tsx | Runs show each step's output, the failed step, the error, the trigger data — not each step's input. |
| 3.6 | Workflow credits | ✗ | ✗ | — | Not built, as planned. |
| 3.6 | AI chatbot, agents, threads on records | ✔ / ≈ | ✔ / ≈ | D-070; assistant/tools.rs | Writes are proposals to apply; an agent step only looks up; chats in a side panel. |
| 3.6 | AI roles; MCP server | ✗ / ≈ | ✗ / ◌ | D-071, D-084 | No roles (one owner); MCP not built (optional, D-071 / D-084). |
| 3.6 | Dashboards | ✔ | ≈ | D-067; crm/record/LayoutEditor.tsx | No Duplicate widget (Duplicate dashboard only); Earliest / Latest aggregates removed; a numeric X axis sorts as text. |
| 3.7 | Data model settings | ✔ | ✔ | D-060; DataModelObject.tsx | — |
| 3.7 | Experience: time zone and formats | ≈ | ≈ | D-072; FormatsGroup.tsx | Only the time zone has "This Mac's"; date, time, number and week start have no "System settings" choice. |
| 3.7 | Workspace name, logo; profile | ✔ | ✔ | WorkspacePage.tsx, GeneralTab.tsx | — |
| 3.7 | Developers: keys, webhooks, playground | ✔ / ≈ | ✔ / ≈ | D-071, D-084; DevelopersPanel.tsx | The playground is a static docs page (no try-it); one key opens Twenty's and Plane's paths. |
| 3.7 | Accounts (emails, calendars) | ≈ | ≈ | D-066; AccountsPanel.tsx | In the app-wide Settings window; the settings are the current workspace's. |
| 3.7 | Members, roles, SSO, billing… | ✗ | ✗ | — | Not built, as planned (one owner). |
| 3.8 | Local REST API, keys, docs, webhooks | ≈ | ≈ | D-071, D-084; api/rest.rs | No GraphQL; no `depth` (relations as {id, label}); createdAt / updatedAt not filterable; cursors are offsets; docs not OpenAPI. |

Taken in 0.12.0: Run workflow on selected rows. Left: Duplicate widget; a Search row in the sidebar; `g s` → Settings.

## Plane (project management) — plan §4

### §4.1–4.3 Workspace and shell, projects, work items
| § | Feature | Plan | Verified | Evidence | Difference or gap |
|---|---|---|---|---|---|
| 4.1 | Workspaces: create, switch, name, logo, slug | ✔ | ✔ | commands/workspace.rs | The logo is the brand pack's (Workspace → Brand); the slug is set at creation, not changed. |
| 4.1 | Home: greeting, widgets, Manage widgets | ✔ | ✔ | D-082; HomePage.tsx | — |
| 4.1 | Your work: counts, workload, pie, activity, tabs | ✔ | ✔ | D-082; YourWorkPage.tsx | Counts in the tab labels. |
| 4.1 | Inbox (notifications) | ✔ / ≈ | ✔ / ≈ | D-075; InboxPage.tsx, pm/inbox.rs | In-app only; preferences hide kinds (nothing is mailed). |
| 4.1 | Drafts, Archives, Favorites, Recents, Stickies, Quick links | ✔ | ≈ | D-079, D-082; Sidebar.tsx, CrmNav.tsx | Favorites live in the CRM section (gone when CRM is off; projects aren't starred there); quick links aren't edited (remove and add). |
| 4.1 | Power K inside ⌘K | ✔ | ≈ | D-082; CommandMenu.tsx | Search and go everywhere; create: work items only (no project / cycle / module / page / view create, no actions on the open item). |
| 4.1 | Keyboard shortcuts | ✔ | ≈ | D-082; useShortcuts.ts | All projects is `g j` (`g p` stays People); N·I / N·C / N·M / N·D on their project tab. |
| 4.1 | Customize navigation | ✔ | ✔ | D-082; PmNav.tsx | — |
| 4.1 | Profile settings | ✔ / ≈ | ✔ / ≈ | D-075, D-084; FormatsGroup, WorkItemsGroup, PlaneDevelopersPanel | Theme = macOS; time zone and notice kinds in Settings → General; activity on Your work; tokens in Workspace → Projects. |
| 4.1 | Members, roles, SSO, billing, admin, live | ✗ | ✗ | — | One owner, as planned. |
| 4.1 | Integrations, outside importers | ✗ | ✗ | — | As planned. |
| 4.1 | Publish a project to the web | ✗ | ✗ | — | As planned. |
| 4.2 | Create a project, then its features | ✔ | ≈ | D-073; pm.ts, projects.rs | Cover is a colour band, logo an emoji; two open projects can't share a name (a done one may). |
| 4.2 | Projects list: cards / list, search, favorites, archived | ✔ / ✗ | ≈ / ✗ | D-073; ProjectsPage.tsx | Filter by network only (no lead / member / date filters); no manual order; no join / leave (one owner). |
| 4.2 | Settings → General | ✔ | ✔ | D-073; ProjectSettings.tsx | — |
| 4.2 | Settings → Members | ≈ | ≈ | D-073; ProjectSettings.tsx | Lead, default assignee and subscribers ("me" or people); no member list or roles. |
| 4.2 | Settings → Features | ✔ | ✔ | ProjectSettings.tsx | — |
| 4.2 | Settings → States | ✔ | ✔ | D-073; pm/settings.rs | — |
| 4.2 | Settings → Labels | ✔ | ✔ | D-073; pm/settings.rs | — |
| 4.2 | Settings → Estimates | ✔ | ✔ | D-073; pm.ts | — |
| 4.2 | Settings → Automations | ✔ | ≈ | D-075, D-078; pm/automations.rs | Runs daily while New Dawn is open; a module completed, cancelled or archived lets its items go before its target date. |
| 4.2 | Settings → Imports (CSV), Exports | ✔ | ≈ | D-083; ProjectPage.tsx, WorkItems.tsx | Not a Settings section: import in the project's ⋯, export on Work items, history in Workspace → CRM → Data model. |
| 4.2 | Paid-tier project features | ◌ | ◌ | — | — |
| 4.2 | The dossier as the Overview tab | ≈ | ≈ | D-073, D-085; ProjectPage.tsx | Ours; it also holds the agent's Plane-side proposals. |
| 4.3 | Work item properties | ✔ / ≈ | ✔ / ≈ | D-074; pm/items.rs | Assignees "me" or people; completion time kept, not who completed. |
| 4.3 | Create modal and quick add | ✔ | ✔ | D-074, D-076; CreateItem.tsx | — |
| 4.3 | Peek modes, full page, header actions | ✔ | ≈ | D-074; ItemView.tsx | No Copy link (Copy ID copies the key); Remove from module added in 0.12.0; "Copy to…" lists eight projects. |
| 4.3 | Sub-items, relations, links, attachments, comments, subscribers | ✔ | ≈ | D-074, D-075; ItemView.tsx | Sub-items added new by title (no "Add existing" there; set a parent from the item instead). |
| 4.3 | Activity tabs | ✔ / ◌ | ≈ / ◌ | D-074; ItemView.tsx | All five tabs; newest / oldest order added in 0.12.0; no activity filters; paid tabs absent. |
| 4.3 | Archive, restore, delete, copy, branch name | ✔ | ≈ | D-074; relations.rs, Archived.tsx | No Copy branch name; a project's archived items searchable from 0.12.0 (the workspace's Archives page lists, without search). |
| 4.3 | Drafts | ✔ | ≈ | D-074; Drafts.tsx | Drafts come from closing the create sheet with something written; no "Draft a work item" button. |
| 4.3 | Bulk operations (List, Table) | ✔ | ✔ | D-075, D-076; BulkBar.tsx | Modules: adds one (doesn't replace the set). |
| 4.3 | Create work items via URL | ≈ | ✗ | — | No `newdawn://` link scheme handled; the local API's POST is the way in. |
| 4.3 | Paid-tier work item features | ◌ | ◌ | — | — |

Taken in 0.12.0: automations keep items in a module under way; Remove from module; activity newest / oldest; search in a project's archives. Left: Favorites shown when CRM is off.

### §4.4–4.5 Layouts and views; cycles, modules, pages, intake, analytics, data
| § | Feature | Plan | Verified | Evidence | Difference or gap |
|---|---|---|---|---|---|
| 4.4 | Layouts: List, Board, Calendar, Table, Timeline (Gantt) | ✔ / ◌ | ✔ / ◌ | D-076; WorkItems.tsx, Board/Calendar/Table/Timeline/Gantt.tsx | List shows sub-items flat, marked ↳; Gantt dependencies and critical path not built (◌). |
| 4.4 | Display properties | ✔ | ✔ | D-076; pm/display.rs | Created on / by and Updated on may also show in List and Board (Plane: Table). |
| 4.4 | Group, sub-group, order, display filters | ✔ | ✔ | D-076; pm/display.rs | — |
| 4.4 | Filters: fields, date operators, Clear all | ✔ | ≈ | D-077; pm/filters.rs | Adds Subscribed by; Mentions matches comment mentions, not description ones. |
| 4.4 | Project and workspace views | ✔ / ≈ | ✔ / ≈ | D-077; WorkspaceViews.tsx | Private / locked are flags, not rights; workspace views: no Display panel, no state / label / cycle / module filters. |
| 4.4 | Display remembered per project and view | ✔ | ≈ | D-076, D-077; WorkItems.tsx | One row a project; a view's display changes wait for Save view; built-in workspace views use fixed defaults. |
| 4.5 | Cycles | ✔ / ◌ | ✔ / ◌ | D-078; Cycles.tsx | Adds a Draft tab; auto-schedule and parallel cycles not built (overlaps refused). |
| 4.5 | Modules | ✔ | ✔ | D-079; pm/modules.rs | — |
| 4.5 | Pages and the page editor | ✔ / ◌ | ≈ / ◌ | D-080; kernel/editor/Editor.tsx | Mermaid stays a code block (plan's fallback); emoji via the macOS picker; paid / web-only parts not built. |
| 4.5 | Intake | ✔ / ◌ | ≈ / ◌ | D-081; items/Intake.tsx | One filter (priority); forms and email intake not built — the Email reading agent files requests. |
| 4.5 | Analytics | ✔ / ◌ | ≈ / ◌ | D-083; AnalyticsPage.tsx | Light Cycles and Modules tabs only; no Projects or Intake tab. |
| 4.5 | Exports CSV / XLSX / JSON, history | ✔ | ≈ | D-083; pm/transfer.rs | One project at a time from Work items; workspace Views has no Export; history shared with the CRM (last 50). |
| 4.5 | Plane CSV importer | ✔ | ≈ | D-083; pm/transfer.rs | No separate migrations table (shared history, counts); row problems shown once in the sheet. |
| 4.5 | Webhooks, API tokens, REST API, MCP | ≈ | ≈ | D-084; api/plane.rs | No MCP; no comments / links / activities / attachments endpoints; no `expand`; no retries; localhost only. |
| 4.5 | Plane AI → New Dawn's agents | ≈ | ≈ | D-085; projects.rs | The assistant (D-070) can't read Projects data — nothing like Plane's Pi chat. |

Left for later: export across projects from workspace views; Intake assignee and label filters; a Display panel for workspace views; a light Intake tab in Analytics; comments, links and activity in the Plane API.
