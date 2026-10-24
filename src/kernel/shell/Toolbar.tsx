import { useMatches } from "@tanstack/react-router";
import RecordPill from "../../modules/recorder/RecordPill";
import styles from "./Toolbar.module.css";

/** Unified 52 px toolbar, transparent over native glass. Search is inert until its feature lands. */
export default function Toolbar({ sidebarHidden, onStartRecording }: { sidebarHidden: boolean; onStartRecording: () => void }) {
  const { title, breadcrumb } = useMatches({
    select: (matches) => matches[matches.length - 1]?.staticData ?? {},
  });

  return (
    <header className={sidebarHidden ? `${styles.toolbar} ${styles.clearLights}` : styles.toolbar} data-tauri-drag-region>
      <div className={styles.titleBlock}>
        {breadcrumb && <span className={styles.breadcrumb}>{breadcrumb} /</span>}
        <span className={styles.title}>{title}</span>
      </div>

      <input className={styles.search} type="search" placeholder="Search   ⌘K" aria-label="Search" disabled />

      <RecordPill onStart={onStartRecording} />
    </header>
  );
}
