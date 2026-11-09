# New Dawn — plan v5: one app, not three (UX/UI refinement)

_Approved 2026-11-09. Plan v4 (`docs/plan-v4.md`) built Twenty's CRM and Plane's Projects inside New Dawn, feature for feature. This plan makes them feel like one app. It changes no data and no AI; it is about the shell, navigation, page grammar and consistency. Nothing in it needs a migration or a new dependency._

---

## 1. What is wrong today (seen in the dev app, 2026-11-09)

1. **The sidebar is three sidebars glued together.** New Dawn's pages (Today … Workspace), then Plane's whole section (Home, Your work, Inbox, Drafts, Views, Analytics, Active cycles, Archives, All projects, each project with its parts), then Twenty's (Favorites, "Workspace" with People, Companies, Opportunities, Notes, Tasks, Dashboards, Workflows, Workflow runs), then Developer, Settings. That is 35+ rows: it scrolls, the first rows leave the screen, and the eye can't find a module quickly.
2. **Module navigation leaks into the app's navigation.** A project's Work items / Cycles / Modules / Views / Pages are sidebar rows *and* tabs on the project page. CRM objects are sidebar rows *and* a 10-tab segmented control on the CRM page. The Projects list page repeats Active cycles / Views / Drafts as header links. The same destination appears two or three times.
3. **Words collide.** "Workspace" is the sidebar item for the workspace's settings *and* the label of the CRM objects section. "Tasks" is New Dawn's list *and* a CRM object. "Home" (Plane) and "Today" (New Dawn) are both front pages. "Inbox" sits under Projects but reads as the app's inbox. "Views" appears four times (Projects rail, project tab, CRM views, header link).
4. **Too many control rows before content.** A project's Work items page stacks: status segmented → 8 tabs → layout segmented + Group by + Order by + Display → Filters row → the list. Four rows of controls.
5. **The toolbar search says "Search meetings" on every page**, including People and Work items. The page title block is "Projects / Home", while the page says "Good afternoon, Tien Nguyen".
6. **Page headers follow three grammars.** Eyebrow "VIFC" (the workspace, already shown by the badge) on most pages; "PROJECT · BBDR" on a project; "MONDAY 9 NOVEMBER" on Today. Primary actions sit sometimes in the header, sometimes in the control row, sometimes both.
7. **Segmented controls carry 8–10 options** (CRM objects, project parts, Workspace settings). Apple's segmented control is for 2–5 choices.

What is right and stays: Apple glass shell, Majoris palette and type (D-033), DM Sans, capsule controls, ghost sections, the Record pill, ⌘K, the AI marker, the dossier rhythm, modules on/off per workspace (D-059).

---

## 2. The rule for this phase

**Two levels, two places.** The **sidebar** holds the app's destinations: one row per module, nothing below a module. Each **module owns its navigation inside its own frame**: a module rail (a second column inside the content area) for modules with many destinations, a segmented control for modules with 2–5. Nothing from inside a module appears in the sidebar — except Favorites, which is the one cross-module shortcut list, and the count badges that mean "needs you".

**One grammar for every page.** Eyebrow = where you are (module, or parent record); title = the page; one meta line; one primary action at the right; then one control row (view switcher · filters · sort · ⋯); then content. Never more than one control row before content; filters that are set show as chips under it.

**Same thing, same name, one place.** Each destination has one name and one way in. Where two modules offer the same thing (tasks, home, inbox) the app keeps one and links the other.

---

## 3. The new shape

```
┌──────────────┬──────────────────┬─────────────────────────────────────────────────┐
│ ● ● ●        │  ‹ ›  Projects / Block B defects repair     [Search work items]  ● Record │
│  [VIFC logo] ├──────────────────┼─────────────────────────────────────────────────┤
│              │ ‹ All projects   │  PROJECT · BBDR                                   │
│  Today     2 │                  │  Block B defects repair          [Active|On hold|Done] ⋯ │
│  Meetings    │  Overview        │  Investigation · Core drilling plan agreed · by 20 Nov    │
│  Live        │  Work items    ● │                                                   │
│  Tasks     2 │  Cycles          │  [List|Board|Calendar|Table|Timeline]  State ▾  Manual ▾  Display  [New work item] │
│  Email       │  Modules         │  Filters: (none) · Save as a view · Export        │
│  Projects  3 │  Views           │                                                   │
│  CRM       8 │  Pages           │  ● Backlog 0  +                                   │
│              │  Intake          │  ┌───────────────────────────────────────────┐    │
│  FAVORITES   │  Archives        │  │ No work items yet — press N, or type one  │    │
│  ★ Block B   │  Settings        │  └───────────────────────────────────────────┘    │
│  ★ Keppel    │                  │                                                   │
│              │                  │                                                   │
│  ⚙ Settings  │                  │                                                   │
│  ⟨⟨ Collapse │                  │                                                   │
└──────────────┴──────────────────┴─────────────────────────────────────────────────┘
   sidebar 240      module rail 220            content (page grammar)
   native glass     --surface-2, hairline       --canvas
```

### 3.1 Sidebar (app level)
- Rows, in order: **Today · Meetings · Live · Tasks · Email · Projects · CRM**. Projects and CRM only when the module is on (D-059). Seven rows; the whole list fits above the fold at any window height.
- Count badges only for what needs you: Tasks (due today or overdue), Projects (updates waiting + unread inbox), CRM (commitments waiting). Same style everywhere (`.count`, `.dot` in the icon rail).
- **Favorites**: one section for pins from any module (records, views, cycles, modules, pages, projects, meetings). Folders as today (D-064), collapsed by default when more than 8 pins. The ✎ customise mode stays.
- **Footer**: Settings (⌘,) and Collapse. The **Workspace page moves out of the sidebar**: it opens from the workspace badge's menu ("Workspace settings…"), from Settings → Workspaces, and from ⌘K. It keeps its route `/workspace`.
- The "Developer" section stays dev-build only (unchanged).
- Keyboard: ⌘1–⌘9 keep switching workspaces (D-064's `g` keys stay for the pages and records).
- Icon rail (76 pt) unchanged in behaviour; with seven rows it never scrolls.

### 3.2 Module rail (module level) — one component, `kernel/shell/ModuleRail.tsx`
- 220 pt column at the left of the content area, `--surface-2` background, 0.5 px hairline on its right, its own scroll. Rows look like sidebar rows (28 pt, radius `--radius-control`, active = primary tint), group labels 11/700 uppercase `--ink-3`.
- Optional **head**: a "‹ Parent" back row when the rail is a drill-down (inside a project; inside a CRM record's object), then the current thing's name.
- Collapses to a 44 pt icon strip whenever the sidebar is in its icon mode (under 1280 pt or by hand, D-036) and hides with it (⌥⌘S): one setting, no second toggle to learn.
- **Used by Projects and CRM.** Today, Meetings, Live, Tasks and Email have no rail: they are one page each (Meetings keeps its filter row; a meeting's tabs stay a segmented control — 4 choices).

### 3.3 Projects module
- **Rail (workspace level)**: Overview (was Plane's "Home": greeting, quick links, recents, stickies) · Your work · Inbox · Drafts · Views · Analytics · Active cycles · Archives; then group **PROJECTS**: each project (emoji/mark + name, star to pin), "+ New project" as the last row. Plane's "Customize navigation" (hide rows) stays, as the rail's ✎.
- **Rail (inside a project)**: head "‹ All projects" + project name; rows Overview · Work items · Cycles · Modules · Views · Pages · Intake (as the project's features say) · Archives · Settings. The project page loses its 8-tab segmented control; the route keeps `?tab=` so links and favorites don't break.
- **Project header**: eyebrow "PROJECT · BBDR", title, meta line, the status segmented (3 choices) and ⋯ at the right. Then **one control row**: layout segmented · Group by · Order by · Display · primary action. Filters set show as chips under it with "Save as a view" and "Export" in the ⋯.
- The Projects list page loses its header links (Active cycles, Views, Drafts) — they are rail rows. Its own control row: search · network · sort · archived · cards/list.
- **Inbox** is Plane's notices and stays inside Projects; Today already counts it. Its route moves to `/projects/inbox` (a redirect from `/inbox` stays).
- A work item's own window (D-074) has no rail.

### 3.4 CRM module
- **Rail**: group **RECORDS**: the objects in the owner's order and folders (People, Companies, Opportunities, Notes, …), hidden ones left out, ✎ as today; group **AUTOMATION**: Dashboards · Workflows · Workflow runs; group **NEW DAWN**: Commitments · Waiting for you; "Settings" row → Workspace → CRM (data model, developers).
- The CRM page loses its 10-tab segmented control. The object's **views** stay as the dropdown at the top of the table ("All People ▾"); the control row is view ▾ · Filter · Sort · ⋯ · primary action ("New person").
- **Record page**: the rail stays on the record's object (so the user sees where they are); head "‹ People" + the record's name. The record page itself is unchanged.
- **Tasks**: the CRM `Tasks` object stays out of the rail (one "Tasks" in the app). New Dawn's Tasks page gets a quiet "Table and board by status ›" link opening `/crm?tab=tasks`; the object stays reachable from ⌘K and `g t`.
- The section label "WORKSPACE" (Twenty's word) goes; nothing in the CRM is called Workspace any more.

### 3.5 Toolbar
- Title block: **module / page** from route `staticData`, as today, but every route gets the right pair (today several say "Projects / Home" while the page says something else). The project's and record's names replace the generic "Project" / "Record".
- **Search field is scoped to the module**: placeholder "Search meetings" on Meetings and Today, "Search people" on a CRM object, "Search work items" in Projects, "Search" elsewhere; Enter runs that module's search (meetings: `?q`; CRM: the object's search; Projects: ⌘K in search mode). ⌘K stays the one command menu.
- Right side unchanged: AI activity, Record pill.

### 3.6 Page grammar — components in `kernel/shell/`
- `PageHeader`: eyebrow · title · meta · `actions` slot (one primary, others in ⋯). Every page uses it; the 17 hand-written headers go.
- `ControlRow`: view switcher (segmented ≤ 5) · filters · sort · ⋯ · primary action; `FilterChips` under it. Used by Meetings, Projects list, Work items, CRM tables, Archives, Analytics, Inbox.
- Eyebrow rule: the module or parent ("MEETINGS", "PROJECT · BBDR", "PEOPLE"), never the workspace name (the badge shows it); Today keeps its date.
- Segmented controls hold 2–5 choices. More → rail, select or ⋯ menu.
- Empty states are ghost cards with one sentence (as the Inbox already does); no page starts with two ghost columns and an input row above the fold (Projects Overview's quick links form moves into "Add link" in its card).

### 3.7 Consistency pass (the whole app)
- One `ListRow` style for Tasks, Today, Inbox, Emails read, Archives (title · second line · right slot) so lists read alike.
- One `Chip` set: neutral, primary tint, status; counts and "waiting" always the same.
- Spacing audit of the 49 CSS modules against the skill's rhythm (8/12/16/24; sections 48; control row 24 under the header): hand-tuned paddings replaced by the shared classes.
- Dark mode, Reduce Transparency and the 1180 pt minimum checked on every page touched; the rail follows the same rules as the sidebar (solid under reduced transparency).
- `newdawn-ui-patterns` skill updated with the rail, the page grammar and the control row; `DECISIONS.md` gets one entry per release.

---

## 4. What does not change
- The data model, routes' meaning (URLs keep working; `/inbox` redirects), the record engine, the Projects schema, the agents.
- The Apple shell and Majoris tokens (D-033); DM Sans; the brand pack stays for documents only.
- Parity with Twenty and Plane in features (`docs/parity.md`): this plan moves where things are, not what they do. Where a Twenty/Plane screen had its own shape (Plane's Home widgets, Twenty's views dropdown) it stays, inside the module.
- No new dependency.

---

## 5. Releases (patch bumps, one sitting each unless noted)

### 0.12.2 — The sidebar carries modules; the module rail; Projects moves in (2 sittings)
- `Sidebar.tsx`: the seven rows, Favorites, footer; `PmNav`/`CrmNav` leave the sidebar. Workspace page reachable from the badge menu, Settings → Workspaces and ⌘K.
- `ModuleRail.tsx` + CSS: rows, groups, head, icon strip, reduced-transparency solid, per-Mac state.
- Projects: workspace-level rail (3.3); the Projects list page loses its header links; Plane's Home renamed Overview; `/inbox` → `/projects/inbox`.
- Acceptance: the sidebar never scrolls on a 1180 × 700 window; every former sidebar destination is one click away in its module; the dev screenshots before/after in `LAST_BUILD.md`.

### 0.12.3 — CRM in its rail; one Tasks
- CRM rail (3.4); the 10-tab segmented control goes; the record page's rail head; the CRM Tasks object hidden by default with the link from Tasks.
- Favorites in the sidebar accept pins from both modules (they do already; verified) and collapse past 8.

### 0.12.4 — Inside a project: the rail drill-down, one control row
- Project rail (3.3), tabs removed, `?tab=` kept; project header and the single control row; filter chips; Save as a view / Export in ⋯.
- Work item page: `PageHeader`; its own window unchanged.

### 0.12.5 — Toolbar and page grammar across the app
- Scoped search; route titles fixed; `PageHeader` and `ControlRow` applied to Today, Meetings, Live, Tasks, Email, Workspace, Archives, Analytics, Inbox, Drafts, Views, Your work.
- Eyebrow rule everywhere.

### 0.12.6 — Consistency pass
- `ListRow`, chips, spacing audit, empty states, dark / reduced transparency / 1180 pt on every page; skill and decisions updated; a "UI audit" table in `LAST_BUILD.md` (page → header ✓ · control row ✓ · rail ✓ · dark ✓ · RT ✓).

Each release follows the usual steps: `cargo check`, `tsc`, dev screenshots light + dark, `LAST_BUILD.md`, a D-0xx entry, architect-reviewer, build-check, publish (never install).

---

## 6. Decisions this plan proposes (recorded as built)
- **D-088** Two levels, two places: the sidebar holds modules; a module's navigation lives in its rail. Favorites is the one cross-module list.
- **D-089** The module rail: a second column inside the content area, 220 pt, `--surface-2`, drill-down heads, icon strip under 1280 pt.
- **D-090** One page grammar: eyebrow = module or parent, one primary action, one control row, filter chips; segmented controls hold 2–5 choices.
- **D-091** Same thing, same name, one place: Workspace settings leave the sidebar; one Tasks; Plane's Home is Projects' Overview; Inbox is Projects'.
- **D-092** The toolbar search is scoped to the module; ⌘K stays the one command menu.

## 7. Open choices (defaults taken unless you say otherwise)
- **Workspace settings entry**: badge menu + Settings + ⌘K (default), or keep a small "Workspace" row in the footer next to Settings.
- **Rail width**: 220 pt (default) or 200 pt.
- **Favorites**: in the sidebar (default) or at the top of each module's rail (module-local favorites, as Twenty has them).
- **Order of the seven rows**: Today · Meetings · Live · Tasks · Email · Projects · CRM (default: New Dawn's own first, the two modules last).
