import { useLocation } from "@tanstack/react-router";
import { useState } from "react";
import { useNoticeCounts } from "../bridge/notices";
import { usePmProjects, type PmProject } from "../bridge/pm";
import { usePmHiddenNavigation, useSetPmNavigation } from "../bridge/pmHome";
import { usePmIntakeCount } from "../bridge/pmIntake";
import { useProjects } from "../bridge/projects";
import { AnalyticsIcon, ArchivesIcon, CyclesIcon, DraftsIcon, HomeIcon, InboxIcon, ProjectsIcon, SettingsIcon, ViewsIcon, YourWorkIcon } from "./icons";
import ModuleRail, { RailGroup, RailLink, RailMark, railStyles as s } from "./ModuleRail";

/** The rail's rows Customize navigation can hide (the projects always show). */
const ITEMS: { key: string; label: string }[] = [
  { key: "home", label: "Overview" },
  { key: "your_work", label: "Your work" },
  { key: "inbox", label: "Inbox" },
  { key: "drafts", label: "Drafts" },
  { key: "views", label: "Views" },
  { key: "analytics", label: "Analytics" },
  { key: "active_cycles", label: "Active cycles" },
  { key: "archives", label: "Archives" },
];

/** The Projects module's rail (D-089; Plane's section of D-082 moved out of the sidebar). At the
 *  workspace: Overview (Plane's Home), Your work, Inbox (unread), Drafts, Views, Analytics,
 *  Active cycles, Archives — each hideable (✎) — then All projects (with the updates waiting for
 *  you) and the projects. Inside a project it drills down (D-090): "‹ All projects", the
 *  project, and its parts — Overview, Work items, Cycles, Modules, Views, Pages, Intake as its
 *  features say, Archives, Settings. */
export default function ProjectsRail({ slug, compact }: { slug: string | undefined; compact: boolean }) {
  const location = useLocation();
  const projects = usePmProjects(slug).data ?? [];
  // `/projects/<id>` and `/projects/<id>/items/<item>`: inside that project.
  const inside = /^\/projects\/([^/]+)(\/items\/)?/.exec(location.pathname);
  const current = inside && !["home", "your-work", "inbox", "drafts", "views", "analytics", "cycles", "archives"].includes(inside[1]) ? projects.find((p) => p.id === inside[1]) : undefined;
  if (current) return <ProjectRail slug={slug} project={current} onItem={!!inside?.[2]} tab={(location.search as { tab?: string }).tab} compact={compact} />;
  return <WorkspaceRail slug={slug} projects={projects.filter((p) => !p.archived_at)} compact={compact} />;
}

function WorkspaceRail({ slug, projects, compact }: { slug: string | undefined; projects: PmProject[]; compact: boolean }) {
  const hidden = usePmHiddenNavigation(slug).data ?? [];
  const setNav = useSetPmNavigation(slug);
  const notices = useNoticeCounts(slug).data;
  const unread = (notices?.all ?? 0) + (notices?.mentions ?? 0);
  const waiting = (useProjects(slug).data ?? []).reduce((n, p) => n + p.waiting, 0);
  const [customizing, setCustomizing] = useState(false);
  const shown = (key: string) => !hidden.includes(key);

  return (
    <ModuleRail label="Projects" compact={compact}>
      <RailGroup
        label="Projects"
        control={
          <button type="button" className={s.pencil} aria-pressed={customizing} aria-label="Customize navigation" title="Customize navigation" onClick={() => setCustomizing((was) => !was)}>
            ✎
          </button>
        }
      >
        {customizing && !compact && (
          <div className={s.customize} role="group" aria-label="Show in the rail">
            {ITEMS.map((i) => (
              <label key={i.key} className={s.check}>
                <input type="checkbox" checked={shown(i.key)} disabled={setNav.isPending} onChange={(e) => setNav.mutate({ key: i.key, hidden: !e.target.checked })} /> {i.label}
              </label>
            ))}
          </div>
        )}
        {shown("home") && <RailLink to="/projects/home" search={{}} activeOptions={{ includeSearch: false }} icon={<HomeIcon />} label="Overview" compact={compact} />}
        {shown("your_work") && <RailLink to="/projects/your-work" search={{}} activeOptions={{ includeSearch: false }} icon={<YourWorkIcon />} label="Your work" compact={compact} />}
        {shown("inbox") && <RailLink to="/projects/inbox" search={{}} activeOptions={{ includeSearch: false }} icon={<InboxIcon />} label="Inbox" count={unread} compact={compact} />}
        {shown("drafts") && <RailLink to="/projects/drafts" icon={<DraftsIcon />} label="Drafts" compact={compact} />}
        {shown("views") && <RailLink to="/projects/views" search={{}} activeOptions={{ includeSearch: false }} icon={<ViewsIcon />} label="Views" compact={compact} />}
        {shown("analytics") && <RailLink to="/projects/analytics" search={{}} activeOptions={{ includeSearch: false }} icon={<AnalyticsIcon />} label="Analytics" compact={compact} />}
        {shown("active_cycles") && <RailLink to="/projects/cycles" icon={<CyclesIcon />} label="Active cycles" compact={compact} />}
        {shown("archives") && <RailLink to="/projects/archives" search={{}} activeOptions={{ includeSearch: false }} icon={<ArchivesIcon />} label="Archives" compact={compact} />}
      </RailGroup>

      <RailGroup label="All projects">
        <RailLink to="/projects" search={{}} activeOptions={{ exact: true, includeSearch: false }} icon={<ProjectsIcon />} label="All projects" count={waiting} compact={compact} />
        {projects.map((p) => (
          <RailLink key={p.id} to="/projects/$projectId" params={{ projectId: p.id }} search={{}} activeOptions={{ includeSearch: false }} icon={<RailMark>{p.emoji || p.identifier.slice(0, 1)}</RailMark>} label={p.name} compact={compact} />
        ))}
      </RailGroup>
    </ModuleRail>
  );
}

/** Inside a project: the way back, the project, its parts. `tab` is the open part (none =
 *  Overview); on a work item's page, Work items. */
function ProjectRail({ slug, project: p, onItem, tab, compact }: { slug: string | undefined; project: PmProject; onItem: boolean; tab: string | undefined; compact: boolean }) {
  const f = p.features;
  const intake = usePmIntakeCount(slug, p.id, !!f.intake).data ?? 0;
  const open = onItem ? "items" : (tab ?? "overview");
  const parts: [string, string, number][] = [
    ["overview", "Overview", 0],
    ["items", "Work items", 0],
    ...(f.cycles ? ([["cycles", "Cycles", 0]] as [string, string, number][]) : []),
    ...(f.modules ? ([["modules", "Modules", 0]] as [string, string, number][]) : []),
    ...(f.views ? ([["views", "Views", 0]] as [string, string, number][]) : []),
    ...(f.pages ? ([["pages", "Pages", 0]] as [string, string, number][]) : []),
    ...(f.intake ? ([["intake", "Intake", intake]] as [string, string, number][]) : []),
    ["archives", "Archives", 0],
    ["settings", "Settings", 0],
  ];
  return (
    <ModuleRail label={p.name} compact={compact}>
      <div className={s.head}>
        <RailLink to="/projects" search={{}} activeOptions={{ exact: true, includeSearch: true }} icon={<span aria-hidden>‹</span>} label="All projects" compact={compact} back />
        <p className={s.headName} title={p.name}>
          {p.emoji && <span aria-hidden>{p.emoji} </span>}
          {p.name}
        </p>
      </div>
      <RailGroup>
        {parts.map(([key, label, count]) => (
          <RailLink
            key={key}
            to="/projects/$projectId"
            params={{ projectId: p.id }}
            search={key === "overview" ? {} : { tab: key }}
            activeOptions={{ exact: true, includeSearch: true }}
            active={open === key}
            icon={key === "settings" ? <SettingsIcon /> : <RailMark>{label.slice(0, 1)}</RailMark>}
            label={label}
            count={count}
            compact={compact}
          />
        ))}
      </RailGroup>
    </ModuleRail>
  );
}
