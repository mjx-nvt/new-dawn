import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { openSettings } from "../bridge/settings";
import { CodeIcon, CollapseIcon, MeetingsIcon, SettingsIcon, WorkspaceIcon } from "./icons";
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

/** Transparent sidebar — native glass shows through. The top strip holds the traffic lights. */
export default function Sidebar({ hidden, iconsOnly, onToggleIcons, switcherOpen, onSwitcherOpenChange }: Props) {
  // In the rail the label stays for screen readers; the tooltip shows it on hover.
  const label = (text: string, icon: ReactNode) => (
    <>
      {icon}
      <span className={iconsOnly ? "srOnly" : styles.label}>{text}</span>
    </>
  );
  const tip = (text: string) => (iconsOnly ? text : undefined);

  return (
    <nav className={iconsOnly ? `${styles.sidebar} ${styles.rail}` : styles.sidebar} inert={hidden} aria-label="Sidebar">
      <div className={styles.dragStrip} data-tauri-drag-region />

      <WorkspaceSwitcher compact={iconsOnly} open={switcherOpen} onOpenChange={onSwitcherOpenChange} />

      <div className={styles.section}>
        <Link to="/meetings" search={{}} className={styles.item} activeProps={{ className: styles.active }} title={tip("Meetings")}>
          {label("Meetings", <MeetingsIcon />)}
        </Link>
        <Link
          to="/workspace"
          search={{ tab: "overview" }}
          activeOptions={{ includeSearch: false }}
          className={styles.item}
          activeProps={{ className: styles.active }}
          title={tip("Workspace")}
        >
          {label("Workspace", <WorkspaceIcon />)}
        </Link>
      </div>

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
