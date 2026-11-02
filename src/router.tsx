import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";
import { getOnboardingDone } from "./kernel/bridge/onboarding";
import type { SettingsTab } from "./kernel/bridge/settings";
import type { SummaryLang } from "./kernel/bridge/summary";
import AppLayout from "./kernel/shell/AppLayout";
import ModuleGate from "./kernel/shell/ModuleGate";
import LivePage from "./modules/live/LivePage";
import MeetingPage, { MEETING_TABS, type MeetingTab } from "./modules/meetings/MeetingPage";
import MeetingsPage, { type MeetingsSearch } from "./modules/meetings/MeetingsPage";
import { SUMMARY_LANGS } from "./modules/meetings/format";
import { SORTS, STATUS_FILTERS, WHEN_FILTERS } from "./modules/meetings/listing";
import WelcomePage, { WELCOME_STEPS, type WelcomeStep } from "./modules/onboarding/WelcomePage";
import TasksPage from "./modules/tasks/TasksPage";
import ProjectsPage from "./modules/projects/ProjectsPage";
import ProjectPage from "./modules/projects/ProjectPage";
import Drafts from "./modules/projects/items/Drafts";
import ItemPage from "./modules/projects/items/ItemPage";
import InboxPage from "./modules/projects/InboxPage";
import WorkspaceViews from "./modules/projects/WorkspaceViews";
import ActiveCycles from "./modules/projects/ActiveCycles";
import CrmPage from "./modules/crm/CrmPage";
import RecordPage from "./modules/crm/record/RecordPage";
import EmailPage from "./modules/email/EmailPage";
import TodayPage from "./modules/today/TodayPage";
import WorkspacePage, { WORKSPACE_TABS, type WorkspaceTab } from "./modules/workspace/WorkspacePage";
import ShellMockup from "./modules/dev/ShellMockup";
import TenantSetupPage, { SETUP_STEPS, type SetupSearch } from "./modules/tenant-setup/TenantSetupPage";
import SettingsPage, { SETTINGS_TABS } from "./modules/workspace-settings/SettingsPage";

// Code-based route tree (D-010). Each route names its toolbar title in staticData.
const rootRoute = createRootRoute({ component: Outlet });

// Main window: sidebar + toolbar + content.
const shellRoute = createRoute({ getParentRoute: () => rootRoute, id: "shell", component: AppLayout });

const indexRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  // First launch (or after New Dawn → Run Setup…): the welcome steps; otherwise the meetings.
  beforeLoad: async () => {
    const done = await getOnboardingDone().catch(() => true);
    throw done ? redirect({ to: "/meetings" }) : redirect({ to: "/welcome", search: { step: "welcome" } });
  },
});

// What needs you today, across every open workspace (D-057).
const todayRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/today",
  component: TodayPage,
  staticData: { title: "Today" },
});

const meetingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/meetings",
  validateSearch: (search: Record<string, unknown>): MeetingsSearch => ({
    ...(search.import === true ? { import: true as const } : {}),
    ...(typeof search.q === "string" && search.q ? { q: search.q } : {}),
    ...(STATUS_FILTERS.includes(search.status as never) && search.status !== "all" ? { status: search.status as MeetingsSearch["status"] } : {}),
    ...(typeof search.type === "string" && search.type ? { type: search.type } : {}),
    ...(WHEN_FILTERS.includes(search.when as never) && search.when !== "all" ? { when: search.when as MeetingsSearch["when"] } : {}),
    ...(SORTS.includes(search.sort as never) && search.sort !== "newest" ? { sort: search.sort as MeetingsSearch["sort"] } : {}),
  }),
  component: MeetingsPage,
  staticData: { title: "Meetings" },
});

const meetingRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/meetings/$meetingId",
  // ?lang= shows the summary in that language instead of English (D-046).
  validateSearch: (search: Record<string, unknown>): { tab: MeetingTab; t?: number; lang?: SummaryLang } => {
    const lang = SUMMARY_LANGS.find((l) => l.id === search.lang)?.id;
    return {
      tab: MEETING_TABS.find((t) => t.id === search.tab)?.id ?? "transcript",
      ...(typeof search.t === "number" && search.t >= 0 ? { t: search.t } : {}),
      ...(lang ? { lang } : {}),
    };
  },
  component: MeetingPage,
  staticData: { title: "Meeting", breadcrumb: "Meetings" },
});

// Live transcription and translation (D-041); ?start=true opens the start sheet, like ?import=true.
const liveRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/live",
  validateSearch: (search: Record<string, unknown>): { start?: true } => (search.start === true ? { start: true as const } : {}),
  component: LivePage,
  staticData: { title: "Live" },
});

// Your to-dos: from meetings' action items and typed (D-042).
const tasksRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/tasks",
  component: TasksPage,
  staticData: { title: "Tasks" },
});

// The projects you follow; the Projects expert proposes their updates after each summary (D-054).
const projectsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects",
  // Plane's list (D-073): the layout, the order and the filters are URL state.
  validateSearch: (search: Record<string, unknown>): { new?: true; layout?: "list"; sort?: "name" | "created" | "identifier"; network?: "private" | "public"; archived?: true } => ({
    ...(search.new === true ? { new: true as const } : {}),
    ...(search.layout === "list" ? { layout: "list" as const } : {}),
    ...(search.sort === "name" || search.sort === "created" || search.sort === "identifier" ? { sort: search.sort } : {}),
    ...(search.network === "private" || search.network === "public" ? { network: search.network } : {}),
    ...(search.archived === true || search.archived === "true" ? { archived: true as const } : {}),
  }),
  component: () => (
    <ModuleGate module="projects">
      <ProjectsPage />
    </ModuleGate>
  ),
  staticData: { title: "Projects" },
});

const projectRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/$projectId",
  // The project's tab and, under Settings, its section (D-073).
  validateSearch: (search: Record<string, unknown>): { tab?: string; section?: string; view?: string; f?: string; layout?: string; group?: string; sub?: string; order?: string; peek?: string; mode?: string; activity?: string; cycle?: string; ctab?: string; cfrom?: string; cto?: string; carch?: string; module?: string; mlayout?: string; msort?: string; mstatus?: string; mlead?: string; mmember?: string; mfav?: string; march?: string; mfrom?: string; mto?: string; page?: string; ptab?: string; psort?: string; pfav?: string; plabel?: string; pfull?: string } => {
    const text = (k: string) => (typeof search[k] === "string" && search[k] ? { [k]: search[k] as string } : {});
    // The tab, Settings' section, the work items' open view, filters, layout, group, sub-group,
    // order, peek and its activity tab, the Cycles tab's own tab and open cycle, the Modules
    // tab's layout, sort, filters and open module, and the Pages tab's tab, sort, filters, open
    // page and full screen (D-073 … D-080).
    return { ...text("tab"), ...text("section"), ...text("view"), ...text("f"), ...text("layout"), ...text("group"), ...text("sub"), ...text("order"), ...text("peek"), ...text("mode"), ...text("activity"), ...text("cycle"), ...text("ctab"), ...text("cfrom"), ...text("cto"), ...text("carch"), ...["module", "mlayout", "msort", "mstatus", "mlead", "mmember", "mfav", "march", "mfrom", "mto", "page", "ptab", "psort", "pfav", "plabel", "pfull"].reduce((all, k) => ({ ...all, ...text(k) }), {}) };
  },
  component: () => (
    <ModuleGate module="projects">
      <ProjectPage />
    </ModuleGate>
  ),
  staticData: { title: "Project", breadcrumb: "Projects" },
});

// A work item's page (D-074) — also a window of its own.
const itemRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/$projectId/items/$itemId",
  // The activity tab (D-074).
  validateSearch: (search: Record<string, unknown>): { activity?: string } => (typeof search.activity === "string" && search.activity ? { activity: search.activity } : {}),
  component: () => (
    <ModuleGate module="projects">
      <ItemPage />
    </ModuleGate>
  ),
  staticData: { title: "Work item", breadcrumb: "Projects" },
});

// Drafts of work items, across the workspace's projects (D-074).
const draftsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/drafts",
  component: () => (
    <ModuleGate module="projects">
      <Drafts />
    </ModuleGate>
  ),
  staticData: { title: "Drafts", breadcrumb: "Projects" },
});

// The workspace's views (D-077): which one, its filters as changed, the peek.
const workspaceViewsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/views",
  validateSearch: (search: Record<string, unknown>): { view?: string; f?: string; peek?: string; mode?: string; activity?: string } => {
    const text = (k: string) => (typeof search[k] === "string" && search[k] ? { [k]: search[k] as string } : {});
    return { ...text("view"), ...text("f"), ...text("peek"), ...text("mode"), ...text("activity") };
  },
  component: () => (
    <ModuleGate module="projects">
      <WorkspaceViews />
    </ModuleGate>
  ),
  staticData: { title: "Views", breadcrumb: "Projects" },
});

// Every open project's running cycle (D-078).
const activeCyclesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/projects/cycles",
  component: () => (
    <ModuleGate module="projects">
      <ActiveCycles />
    </ModuleGate>
  ),
  staticData: { title: "Active cycles", breadcrumb: "Projects" },
});

// The Inbox (D-075): Plane's notifications — the tab, what it shows, the type filters, the work
// item open beside the list and its activity tab.
const inboxRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/inbox",
  validateSearch: (search: Record<string, unknown>): { tab?: string; show?: string; filter?: string; notice?: string; item?: string; activity?: string } => {
    const text = (k: string) => (typeof search[k] === "string" && search[k] ? { [k]: search[k] as string } : {});
    return { ...text("tab"), ...text("show"), ...text("filter"), ...text("notice"), ...text("item"), ...text("activity") };
  },
  component: () => (
    <ModuleGate module="projects">
      <InboxPage />
    </ModuleGate>
  ),
  staticData: { title: "Inbox" },
});

// The CRM module (D-059, D-060): Twenty's objects as tabs, and the commitments (D-055).
const peopleRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/crm",
  // `import` / `export` open the import sheet or run the view's export once, then clear (D-065).
  validateSearch: (search: Record<string, unknown>): { tab: string; view?: string; import?: boolean; export?: boolean } => ({
    tab: typeof search.tab === "string" && search.tab ? search.tab : "people",
    ...(typeof search.view === "string" && search.view ? { view: search.view } : {}),
    ...(search.import === true || search.import === "true" ? { import: true } : {}),
    ...(search.export === true || search.export === "true" ? { export: true } : {}),
  }),
  component: () => (
    <ModuleGate module="crm">
      <CrmPage />
    </ModuleGate>
  ),
  staticData: { title: "CRM" },
});

// A record's page (D-062): any object by its plural name, the tab as URL state.
const recordRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/crm/$object/$recordId",
  validateSearch: (search: Record<string, unknown>): { tab?: string } => (typeof search.tab === "string" && search.tab ? { tab: search.tab } : {}),
  component: () => (
    <ModuleGate module="crm">
      <RecordPage />
    </ModuleGate>
  ),
  staticData: { title: "Record", breadcrumb: "CRM" },
});

// The People routes of 0.9.x, kept for old links.
const peopleRedirect = createRoute({
  getParentRoute: () => shellRoute,
  path: "/people",
  beforeLoad: () => {
    throw redirect({ to: "/crm", search: { tab: "people" } });
  },
});

const personRedirect = createRoute({
  getParentRoute: () => shellRoute,
  path: "/people/$personId",
  beforeLoad: ({ params }) => {
    throw redirect({ to: "/crm/$object/$recordId", params: { object: "people", recordId: params.personId } });
  },
});

// Emails you give New Dawn to read: tasks, project updates, commitments (D-056).
const emailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/email",
  component: EmailPage,
  staticData: { title: "Email" },
});

const workspaceRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/workspace",
  validateSearch: (search: Record<string, unknown>): { tab: WorkspaceTab; object?: string } => ({
    tab: WORKSPACE_TABS.find((t) => t.id === search.tab)?.id ?? "overview",
    // The CRM tab's data model: the object open (D-060, 0.10.5).
    ...(typeof search.object === "string" && search.object ? { object: search.object } : {}),
  }),
  component: WorkspacePage,
  staticData: { title: "Workspace" },
});

const setupRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/setup",
  // The steps after the first need the workspace it made; without one, start there.
  validateSearch: (search: Record<string, unknown>): SetupSearch => {
    const slug = typeof search.slug === "string" && search.slug ? search.slug : undefined;
    return { step: (slug && SETUP_STEPS.find((s) => s.id === search.step)?.id) || "identity", slug };
  },
  component: TenantSetupPage,
  staticData: { title: "Tenant setup" },
});

const devShellRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/dev/shell",
  component: ShellMockup,
  staticData: { title: "Shell mockup", breadcrumb: "Developer" },
});

// First-run setup in the main window: its own layout (no sidebar), step in the URL.
const welcomeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/welcome",
  validateSearch: (search: Record<string, unknown>): { step: WelcomeStep } => ({
    step: WELCOME_STEPS.find((s) => s.id === search.step)?.id ?? "welcome",
  }),
  component: WelcomePage,
});

// Settings window: its own layout, tab kept in the URL (?tab=).
const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  validateSearch: (search: Record<string, unknown>): { tab: SettingsTab } => ({
    tab: SETTINGS_TABS.find((t) => t.id === search.tab)?.id ?? "general",
  }),
  component: SettingsPage,
});

export const router = createRouter({
  routeTree: rootRoute.addChildren([
    shellRoute.addChildren([indexRoute, todayRoute, meetingsRoute, meetingRoute, liveRoute, tasksRoute, projectsRoute, projectRoute, itemRoute, draftsRoute, workspaceViewsRoute, activeCyclesRoute, inboxRoute, peopleRoute, recordRoute, peopleRedirect, personRedirect, emailRoute, workspaceRoute, setupRoute, ...(import.meta.env.DEV ? [devShellRoute] : [])]),
    welcomeRoute,
    settingsRoute,
  ]),
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
  interface StaticDataRouteOption {
    title?: string;
    breadcrumb?: string;
  }
}
