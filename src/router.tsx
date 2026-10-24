import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";
import { getOnboardingDone } from "./kernel/bridge/onboarding";
import type { SettingsTab } from "./kernel/bridge/settings";
import AppLayout from "./kernel/shell/AppLayout";
import MeetingPage, { MEETING_TABS, type MeetingTab } from "./modules/meetings/MeetingPage";
import MeetingsPage from "./modules/meetings/MeetingsPage";
import WelcomePage, { WELCOME_STEPS, type WelcomeStep } from "./modules/onboarding/WelcomePage";
import ShellMockup from "./modules/dev/ShellMockup";
import TenantSetupPage, { SETUP_STEPS, type SetupStep } from "./modules/tenant-setup/TenantSetupPage";
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
  validateSearch: (search: Record<string, unknown>): { import?: true; q?: string } => ({
    ...(search.import === true ? { import: true as const } : {}),
    ...(typeof search.q === "string" && search.q ? { q: search.q } : {}),
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

const setupRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/setup",
  validateSearch: (search: Record<string, unknown>): { step: SetupStep } => ({
    step: SETUP_STEPS.find((s) => s.id === search.step)?.id ?? "identity",
  }),
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
    shellRoute.addChildren([indexRoute, meetingsRoute, meetingRoute, setupRoute, ...(import.meta.env.DEV ? [devShellRoute] : [])]),
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
