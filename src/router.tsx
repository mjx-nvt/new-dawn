import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";
import { getOnboardingDone } from "./kernel/bridge/onboarding";
import type { SettingsTab } from "./kernel/bridge/settings";
import AppLayout from "./kernel/shell/AppLayout";
import LivePage from "./modules/live/LivePage";
import MeetingPage, { MEETING_TABS, type MeetingTab } from "./modules/meetings/MeetingPage";
import MeetingsPage, { type MeetingsSearch } from "./modules/meetings/MeetingsPage";
import { SORTS, STATUS_FILTERS, WHEN_FILTERS } from "./modules/meetings/listing";
import WelcomePage, { WELCOME_STEPS, type WelcomeStep } from "./modules/onboarding/WelcomePage";
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
  validateSearch: (search: Record<string, unknown>): { tab: MeetingTab; t?: number } => ({
    tab: MEETING_TABS.find((t) => t.id === search.tab)?.id ?? "transcript",
    ...(typeof search.t === "number" && search.t >= 0 ? { t: search.t } : {}),
  }),
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

const workspaceRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/workspace",
  validateSearch: (search: Record<string, unknown>): { tab: WorkspaceTab } => ({
    tab: WORKSPACE_TABS.find((t) => t.id === search.tab)?.id ?? "overview",
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
    shellRoute.addChildren([indexRoute, meetingsRoute, meetingRoute, liveRoute, workspaceRoute, setupRoute, ...(import.meta.env.DEV ? [devShellRoute] : [])]),
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
