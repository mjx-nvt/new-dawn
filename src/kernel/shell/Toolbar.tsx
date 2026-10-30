import { useMatches, useNavigate, useRouterState } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import RecordPill from "../../modules/recorder/RecordPill";
import AiActivity from "./AiActivity";
import { BackIcon, ForwardIcon } from "./icons";
import styles from "./Toolbar.module.css";
import { useHistoryNav } from "./useHistoryNav";

/** Unified 52 px toolbar, transparent over native glass: back / forward, title, search, record. */
export default function Toolbar({ sidebarHidden, onStartRecording }: { sidebarHidden: boolean; onStartRecording: () => void }) {
  const { title, breadcrumb } = useMatches({
    select: (matches) => matches[matches.length - 1]?.staticData ?? {},
  });
  const nav = useHistoryNav();
  const navigate = useNavigate();
  const search = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  // Follow the Meetings page's own search (?q), so the field never shows a stale query.
  const location = useRouterState({ select: (s) => s.location });
  const onMeetings = location.pathname === "/meetings";
  const urlQuery = onMeetings ? ((location.search as { q?: string }).q ?? "") : null;
  useEffect(() => {
    if (urlQuery !== null) setQuery(urlQuery);
  }, [urlQuery]);

  // ⌘K focuses search from anywhere in the window.
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const plain = !event.shiftKey && !event.altKey && !event.ctrlKey && !event.isComposing;
      if (event.metaKey && plain && event.key.toLowerCase() === "k" && !document.querySelector("dialog[open]")) {
        event.preventDefault();
        search.current?.focus();
        search.current?.select();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <header className={sidebarHidden ? `${styles.toolbar} ${styles.clearLights}` : styles.toolbar} data-tauri-drag-region="deep">
      <div className={styles.lead}>
        <div className={styles.history}>
          <button type="button" className={styles.navButton} disabled={!nav.canBack} title="Back (⌘[)" aria-label="Back" onClick={nav.back}>
            <BackIcon />
          </button>
          <button
            type="button"
            className={styles.navButton}
            disabled={!nav.canForward}
            title="Forward (⌘])"
            aria-label="Forward"
            onClick={nav.forward}
          >
            <ForwardIcon />
          </button>
        </div>
        <div className={styles.titleBlock}>
          {breadcrumb && <span className={styles.breadcrumb}>{breadcrumb} /</span>}
          <span className={styles.title}>{title}</span>
        </div>
      </div>

      <form
        className={styles.searchForm}
        role="search"
        onSubmit={(e) => {
          e.preventDefault();
          const q = query.trim();
          void navigate({ to: "/meetings", search: q ? { q } : {}, replace: onMeetings });
          search.current?.blur();
        }}
      >
        <input
          ref={search}
          className={styles.search}
          type="search"
          value={query}
          placeholder="Search meetings"
          title="Search meetings (⌘K)"
          aria-label="Search meetings"
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && e.currentTarget.blur()}
        />
      </form>

      <div className={styles.trail}>
        <AiActivity />
        <RecordPill onStart={onStartRecording} />
      </div>
    </header>
  );
}
