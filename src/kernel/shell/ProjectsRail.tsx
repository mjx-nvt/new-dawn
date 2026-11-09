import { useLocation } from "@tanstack/react-router";
import { useState } from "react";
import { useNoticeCounts } from "../bridge/notices";
import { usePmProjects } from "../bridge/pm";
import { usePmHiddenNavigation, useSetPmNavigation } from "../bridge/pmHome";
import { useProjects } from "../bridge/projects";
import { AnalyticsIcon, ArchivesIcon, CyclesIcon, DraftsIcon, HomeIcon, InboxIcon, ProjectsIcon, ViewsIcon, YourWorkIcon } from "./icons";
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

/** The Projects module's rail (D-089; Plane's section of D-082 moved out of the sidebar): Overview
 *  (Plane's Home), Your work, Inbox (unread), Drafts, Views, Analytics, Active cycles, Archives —
 *  each hideable (✎) — then All projects (with the updates waiting for you) and the projects,
 *  each opening onto its parts (Work items, Cycles, Modules, Views, Pages, Intake as its
 *  features say). */
export default function ProjectsRail({ slug, compact }: { slug: string | undefined; compact: boolean }) {
  const location = useLocation();
  const hidden = usePmHiddenNavigation(slug).data ?? [];
  const setNav = useSetPmNavigation(slug);
  const notices = useNoticeCounts(slug).data;
  const unread = (notices?.all ?? 0) + (notices?.mentions ?? 0);
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  const waiting = (useProjects(slug).data ?? []).reduce((n, p) => n + p.waiting, 0);
  const [customizing, setCustomizing] = useState(false);
  const [opened, setOpened] = useState<string[]>([]);
  // The project on screen opens by itself, unless closed by hand.
  const [closed, setClosed] = useState<string[]>([]);
  const current = /^\/projects\/([^/]+)/.exec(location.pathname)?.[1];
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
        {projects.map((p) => {
          const open = (opened.includes(p.id) || current === p.id) && !closed.includes(p.id);
          const toggle = () => {
            setOpened(open ? opened.filter((x) => x !== p.id) : [...opened, p.id]);
            setClosed(open ? [...closed, p.id] : closed.filter((x) => x !== p.id));
          };
          const parts: [string, string][] = [
            ["items", "Work items"],
            ...(p.features.cycles ? ([["cycles", "Cycles"]] as [string, string][]) : []),
            ...(p.features.modules ? ([["modules", "Modules"]] as [string, string][]) : []),
            ...(p.features.views ? ([["views", "Views"]] as [string, string][]) : []),
            ...(p.features.pages ? ([["pages", "Pages"]] as [string, string][]) : []),
            ...(p.features.intake ? ([["intake", "Intake"]] as [string, string][]) : []),
          ];
          return (
            <div key={p.id}>
              <div className={s.twig}>
                {!compact && (
                  <button type="button" className={s.disclosure} aria-expanded={open} aria-label={open ? `Close ${p.name}` : `Open ${p.name}`} onClick={toggle}>
                    {open ? "▾" : "▸"}
                  </button>
                )}
                <RailLink to="/projects/$projectId" params={{ projectId: p.id }} search={{}} activeOptions={{ includeSearch: true }} icon={<RailMark>{p.emoji || p.identifier.slice(0, 1)}</RailMark>} label={p.name} compact={compact} />
              </div>
              {open &&
                !compact &&
                parts.map(([tab, text]) => <RailLink key={tab} to="/projects/$projectId" params={{ projectId: p.id }} search={{ tab }} label={text} compact={compact} part />)}
            </div>
          );
        })}
      </RailGroup>
    </ModuleRail>
  );
}
