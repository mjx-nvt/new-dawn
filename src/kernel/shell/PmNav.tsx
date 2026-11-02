import { Link, useLocation } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";
import { useNoticeCounts } from "../bridge/notices";
import { usePmProjects } from "../bridge/pm";
import { usePmHiddenNavigation, useSetPmNavigation } from "../bridge/pmHome";
import { useProjects } from "../bridge/projects";
import { AnalyticsIcon, ArchivesIcon, CyclesIcon, DraftsIcon, HomeIcon, InboxIcon, ProjectsIcon, ViewsIcon, YourWorkIcon } from "./icons";
import nav from "./CrmNav.module.css";
import styles from "./Sidebar.module.css";

/** The section's items Customize navigation can hide (the projects always show). */
const ITEMS: { key: string; label: string }[] = [
  { key: "home", label: "Home" },
  { key: "your_work", label: "Your work" },
  { key: "inbox", label: "Inbox" },
  { key: "drafts", label: "Drafts" },
  { key: "views", label: "Views" },
  { key: "analytics", label: "Analytics" },
  { key: "active_cycles", label: "Active cycles" },
  { key: "archives", label: "Archives" },
];

/** The sidebar's Projects section, as Plane's (D-082): Home, Your work, Inbox (unread), Drafts,
 *  Views, Active cycles, Archives — each hideable (Customize navigation, ✎) — then the projects,
 *  each opening onto its parts (Work items, Cycles, Modules, Views, Pages, Intake as its features
 *  say), and All projects (with the updates waiting for you). */
export default function PmNav({ slug, iconsOnly }: { slug: string | undefined; iconsOnly: boolean }) {
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
  const label = (text: string, icon: ReactNode) => (
    <>
      {icon}
      <span className={iconsOnly ? "srOnly" : styles.label}>{text}</span>
    </>
  );
  const tip = (text: string) => (iconsOnly ? text : undefined);
  const count = (n: number) => n > 0 && (iconsOnly ? <span className={styles.dot} /> : <span className={styles.count}>{n}</span>);

  return (
    <div className={styles.section} aria-label="Projects">
      {!iconsOnly && (
        <div className={nav.head}>
          <p className={styles.sectionLabel}>Projects</p>
          <button type="button" className={nav.pencil} aria-pressed={customizing} aria-label="Customize navigation" title="Customize navigation" onClick={() => setCustomizing(!customizing)}>
            ✎
          </button>
        </div>
      )}
      {customizing && !iconsOnly && (
        <div className={nav.customize} role="group" aria-label="Show in the sidebar">
          {ITEMS.map((i) => (
            <label key={i.key} className={nav.check}>
              <input type="checkbox" checked={shown(i.key)} disabled={setNav.isPending} onChange={(e) => setNav.mutate({ key: i.key, hidden: !e.target.checked })} /> {i.label}
            </label>
          ))}
        </div>
      )}
      {shown("home") && (
        <Link to="/projects/home" className={styles.item} activeProps={{ className: styles.active }} title={tip("Home")}>
          {label("Home", <HomeIcon />)}
        </Link>
      )}
      {shown("your_work") && (
        <Link to="/projects/your-work" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("Your work")}>
          {label("Your work", <YourWorkIcon />)}
        </Link>
      )}
      {shown("inbox") && (
        <Link to="/inbox" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip(unread ? `Inbox · ${unread} unread` : "Inbox")} aria-label={unread ? `Inbox, ${unread} unread` : undefined}>
          {label("Inbox", <InboxIcon />)}
          {count(unread)}
        </Link>
      )}
      {shown("drafts") && (
        <Link to="/projects/drafts" className={styles.item} activeProps={{ className: styles.active }} title={tip("Drafts")}>
          {label("Drafts", <DraftsIcon />)}
        </Link>
      )}
      {shown("views") && (
        <Link to="/projects/views" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("Views")}>
          {label("Views", <ViewsIcon />)}
        </Link>
      )}
      {shown("analytics") && (
        <Link to="/projects/analytics" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("Analytics")}>
          {label("Analytics", <AnalyticsIcon />)}
        </Link>
      )}
      {shown("active_cycles") && (
        <Link to="/projects/cycles" className={styles.item} activeProps={{ className: styles.active }} title={tip("Active cycles")}>
          {label("Active cycles", <CyclesIcon />)}
        </Link>
      )}
      {shown("archives") && (
        <Link to="/projects/archives" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("Archives")}>
          {label("Archives", <ArchivesIcon />)}
        </Link>
      )}
      <Link
        to="/projects"
        activeOptions={{ exact: true, includeSearch: false }}
        className={styles.item}
        activeProps={{ className: styles.active }}
        title={tip(waiting ? `All projects · ${waiting} updates waiting` : "All projects")}
        aria-label={waiting ? `All projects, ${waiting} updates waiting` : undefined}
      >
        {label("All projects", <ProjectsIcon />)}
        {count(waiting)}
      </Link>
      {!iconsOnly &&
        projects.map((p) => {
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
              <div className={nav.projectRow}>
                <button type="button" className={nav.disclosure} aria-expanded={open} aria-label={open ? `Close ${p.name}` : `Open ${p.name}`} onClick={toggle}>
                  {open ? "▾" : "▸"}
                </button>
                <Link to="/projects/$projectId" params={{ projectId: p.id }} search={{}} activeOptions={{ includeSearch: false }} className={`${styles.item} ${nav.project}`} activeProps={{ className: styles.active }}>
                  <span className={nav.mark} aria-hidden>
                    {p.emoji || p.identifier.slice(0, 1)}
                  </span>
                  <span className={styles.label}>{p.name}</span>
                </Link>
              </div>
              {open &&
                parts.map(([tab, text]) => (
                  <Link
                    key={tab}
                    to="/projects/$projectId"
                    params={{ projectId: p.id }}
                    search={{ tab }}
                    className={`${styles.item} ${nav.part}`}
                    activeProps={{ className: nav.partOn }}
                  >
                    <span className={styles.label}>{text}</span>
                  </Link>
                ))}
            </div>
          );
        })}
    </div>
  );
}
