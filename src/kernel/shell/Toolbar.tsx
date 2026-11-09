import { useMatches, useNavigate, useRouterState } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import RecordPill from "../../modules/recorder/RecordPill";
import { useObjects } from "../bridge/records";
import { useCurrentWorkspace } from "../bridge/workspaces";
import AiActivity from "./AiActivity";
import { BackIcon, ForwardIcon } from "./icons";
import { useCurrentPageTitle } from "./pageTitle";
import styles from "./Toolbar.module.css";
import { useHistoryNav } from "./useHistoryNav";

/** What the search field searches, by where you are (D-092). */
type Scope = { placeholder: string; kind: "meetings" | "crm" | "projects" | "none"; tab?: string };

/** Unified 52 px toolbar, transparent over native glass: back / forward, title, search, record.
 *  The search is the module's: meetings on Meetings and Today, the open object's records in the
 *  CRM, work items and projects in Projects (through ⌘K); elsewhere it searches meetings. ⌘K
 *  stays the one command menu (D-064). */
export default function Toolbar({ sidebarHidden, onStartRecording, onSearchCommand }: { sidebarHidden: boolean; onStartRecording: () => void; onSearchCommand: (query: string) => void }) {
  const { title: routeTitle, breadcrumb, rail } = useMatches({
    select: (matches) => matches[matches.length - 1]?.staticData ?? {},
  });
  const pageTitle = useCurrentPageTitle();
  const title = pageTitle ?? routeTitle;
  const nav = useHistoryNav();
  const navigate = useNavigate();
  const search = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const location = useRouterState({ select: (s) => s.location });
  const { data: workspace } = useCurrentWorkspace();
  const objects = useObjects(rail === "crm" ? workspace?.slug : undefined).data;

  const urlSearch = location.search as { q?: string; tab?: string };
  const onMeetings = location.pathname === "/meetings";
  const onCrmList = location.pathname === "/crm" && !["commitments", "waiting"].includes(urlSearch.tab ?? "");
  // A record's page searches its object (the list it came from).
  const recordObject = /^\/crm\/([^/]+)\//.exec(location.pathname)?.[1];
  const crmTab = onCrmList ? (urlSearch.tab ?? "people") : recordObject;
  const scope: Scope = crmTab
    ? { kind: "crm", tab: crmTab, placeholder: `Search ${(objects?.find((o) => o.name_plural === crmTab)?.label_plural ?? "records").toLowerCase()}` }
    : rail === "projects"
      ? { kind: "projects", placeholder: "Search work items and projects" }
      : { kind: "meetings", placeholder: "Search meetings" };

  // Follow the page's own search (?q), so the field never shows a stale query.
  const urlQuery = onMeetings || onCrmList ? (urlSearch.q ?? "") : null;
  useEffect(() => {
    if (urlQuery !== null) setQuery(urlQuery);
    else setQuery("");
  }, [urlQuery, location.pathname]);

  const submit = () => {
    const q = query.trim();
    if (scope.kind === "crm") {
      const rest = location.search as { view?: string };
      void navigate({ to: "/crm", search: { tab: scope.tab ?? "people", ...(onCrmList && rest.view ? { view: rest.view } : {}), ...(q ? { q } : {}) }, replace: onCrmList });
    } else if (scope.kind === "projects") {
      if (q) onSearchCommand(q);
    } else {
      void navigate({ to: "/meetings", search: q ? { q } : {}, replace: onMeetings });
    }
    search.current?.blur();
  };

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
          submit();
        }}
      >
        <input
          ref={search}
          className={styles.search}
          type="search"
          value={query}
          placeholder={scope.placeholder}
          title={scope.placeholder}
          aria-label={scope.placeholder}
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
