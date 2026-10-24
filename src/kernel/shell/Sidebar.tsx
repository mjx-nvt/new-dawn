import { Link } from "@tanstack/react-router";
import { openSettings } from "../bridge/settings";
import styles from "./Sidebar.module.css";
import WorkspaceSwitcher from "./WorkspaceSwitcher";

interface Props {
  hidden: boolean;
  switcherOpen: boolean;
  onSwitcherOpenChange: (open: boolean) => void;
}

/** Transparent sidebar — native glass shows through. The top strip holds the traffic lights. */
export default function Sidebar({ hidden, switcherOpen, onSwitcherOpenChange }: Props) {
  return (
    <nav className={styles.sidebar} inert={hidden} aria-hidden={hidden}>
      <div className={styles.dragStrip} data-tauri-drag-region />

      <WorkspaceSwitcher open={switcherOpen} onOpenChange={onSwitcherOpenChange} />

      <div className={styles.section}>
        <Link to="/meetings" search={{}} className={styles.item} activeProps={{ className: styles.active }}>
          Meetings
        </Link>
        {/* Inert until the recorder lands (P2.2). */}
        <span className={`${styles.item} ${styles.disabled}`} aria-disabled="true">
          <span className={styles.recordDot} />
          Record
        </span>
      </div>

      {import.meta.env.DEV && (
        <div className={styles.section}>
          <p className={styles.sectionLabel}>Developer</p>
          <Link to="/dev/shell" className={styles.item} activeProps={{ className: styles.active }}>
            Shell mockup
          </Link>
        </div>
      )}

      <div className={styles.footer}>
        <button type="button" className={styles.item} onClick={() => void openSettings("general")}>
          Settings
        </button>
      </div>
    </nav>
  );
}
