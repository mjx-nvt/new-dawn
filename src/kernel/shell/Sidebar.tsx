import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { openSettings } from "../bridge/settings";
import { useOpenTaskCount } from "../bridge/tasks";
import { useCurrentWorkspace } from "../bridge/workspaces";
import { useWaitingCommitments } from "../bridge/crm";
import { ALL_MODULES, useModules } from "../bridge/modules";
import { useNoticeCounts } from "../bridge/notices";
import { useProjects } from "../bridge/projects";
import Favorites from "./Favorites";
import { CodeIcon, CollapseIcon, LiveIcon, MailIcon, MeetingsIcon, PeopleIcon, ProjectsIcon, SettingsIcon, TasksIcon, TodayIcon } from "./icons";
import styles from "./Sidebar.module.css";
import WorkspaceSwitcher from "./WorkspaceSwitcher";

interface Props {
  hidden: boolean;
  /** The 76 pt rail: icons with tooltips, no labels. */
  iconsOnly: boolean;
  onToggleIcons: () => void;
  switcherOpen: boolean;
  onSwitcherOpenChange: (open: boolean) => void;
}

/** Transparent sidebar — native glass shows through. The top strip holds the traffic lights.
 *  Two levels, two places (D-088): the sidebar holds the app's modules, one row each, and the
 *  Favorites; a module's own navigation lives in its rail. Counts mark what needs you. */
export default function Sidebar({ hidden, iconsOnly, onToggleIcons, switcherOpen, onSwitcherOpenChange }: Props) {
  // In the rail the label stays for screen readers; the tooltip shows it on hover.
  const label = (text: string, icon: ReactNode) => (
    <>
      {icon}
      <span className={iconsOnly ? "srOnly" : styles.label}>{text}</span>
    </>
  );
  const tip = (text: string, n = 0) => (iconsOnly ? (n ? `${text} · ${n}` : text) : undefined);
  const count = (n: number) => n > 0 && (iconsOnly ? <span className={styles.dot} /> : <span className={styles.count}>{n}</span>);
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const { data: due = 0 } = useOpenTaskCount(slug); // due today or overdue
  const owed = useWaitingCommitments(slug).data?.length ?? 0; // commitments waiting for you
  const modules = useModules(slug).data ?? ALL_MODULES; // which modules this workspace has on (D-059)
  const notices = useNoticeCounts(modules.projects ? slug : undefined).data;
  const projectsNeed = (useProjects(modules.projects ? slug : undefined).data ?? []).reduce((n, p) => n + p.waiting, 0) + (notices?.all ?? 0) + (notices?.mentions ?? 0);

  return (
    <nav className={iconsOnly ? `${styles.sidebar} ${styles.rail}` : styles.sidebar} inert={hidden} aria-label="Sidebar">
      <div className={styles.dragStrip} data-tauri-drag-region />

      <WorkspaceSwitcher compact={iconsOnly} open={switcherOpen} onOpenChange={onSwitcherOpenChange} />

      <div className={styles.section}>
        <Link to="/today" className={styles.item} activeProps={{ className: styles.active }} title={tip("Today")}>
          {label("Today", <TodayIcon />)}
        </Link>
        <Link to="/meetings" search={{}} className={styles.item} activeProps={{ className: styles.active }} title={tip("Meetings")}>
          {label("Meetings", <MeetingsIcon />)}
        </Link>
        <Link to="/live" search={{}} className={styles.item} activeProps={{ className: styles.active }} title={tip("Live")}>
          {label("Live", <LiveIcon />)}
        </Link>
        <Link to="/tasks" className={styles.item} activeProps={{ className: styles.active }} title={tip("Tasks", due)} aria-label={due ? `Tasks, ${due} due` : undefined}>
          {label("Tasks", <TasksIcon />)}
          {count(due)}
        </Link>
        <Link to="/email" className={styles.item} activeProps={{ className: styles.active }} title={tip("Email")}>
          {label("Email", <MailIcon />)}
        </Link>
        {modules.projects && (
          <Link to="/projects" search={{}} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("Projects", projectsNeed)} aria-label={projectsNeed ? `Projects, ${projectsNeed} waiting for you` : undefined}>
            {label("Projects", <ProjectsIcon />)}
            {count(projectsNeed)}
          </Link>
        )}
        {modules.crm && (
          <Link to="/crm" search={{ tab: "people" }} activeOptions={{ includeSearch: false }} className={styles.item} activeProps={{ className: styles.active }} title={tip("CRM", owed)} aria-label={owed ? `CRM, ${owed} commitments waiting` : undefined}>
            {label("CRM", <PeopleIcon />)}
            {count(owed)}
          </Link>
        )}
      </div>

      {(modules.crm || modules.projects) && <Favorites slug={slug} iconsOnly={iconsOnly} />}

      {import.meta.env.DEV && (
        <div className={styles.section}>
          {!iconsOnly && <p className={styles.sectionLabel}>Developer</p>}
          <Link to="/dev/shell" className={styles.item} activeProps={{ className: styles.active }} title={tip("Shell mockup")}>
            {label("Shell mockup", <CodeIcon />)}
          </Link>
        </div>
      )}

      <div className={styles.footer}>
        <button type="button" className={styles.item} title={tip("Settings (⌘,)")} onClick={() => void openSettings("general")}>
          {label("Settings", <SettingsIcon />)}
        </button>
        <button
          type="button"
          className={styles.item}
          title={iconsOnly ? "Show labels (⌃⌘S)" : "Icons only (⌃⌘S)"}
          aria-label={iconsOnly ? "Show sidebar labels" : "Collapse sidebar to icons"}
          onClick={onToggleIcons}
        >
          <CollapseIcon expand={iconsOnly} />
          {!iconsOnly && <span className={styles.label}>Collapse</span>}
        </button>
      </div>
    </nav>
  );
}
