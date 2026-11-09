import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { openLink } from "../../kernel/bridge/pmItems";
import { usePmHomeWidgets, usePmQuickLinks, usePmRecents, usePmStickies, useQuickLinkMutations, useSetPmHomeWidgets, useStickyMutations, type HomeWidget, type PmRecent, type RecentKind } from "../../kernel/bridge/pmHome";
import { useProfile } from "../../kernel/bridge/profile";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import Editor from "../../kernel/editor/Editor";
import { formatDateTime, formatDay, formatTime } from "../../kernel/format/format";
import DangerButton from "../../kernel/shell/DangerButton";
import OnOff from "../workspace-settings/OnOff";
import styles from "./items/Items.module.css";

const WIDGET_LABEL: Record<string, string> = { quick_links: "Quick links", recents: "Recents", stickies: "Your stickies" };
const RECENT_TABS: { key: RecentKind | "all"; label: string }[] = [
  { key: "all", label: "All" },
  { key: "item", label: "Work items" },
  { key: "page", label: "Pages" },
  { key: "project", label: "Projects" },
];
const STICKY_COLORS = ["yellow", "green", "blue", "purple", "red", "gray"] as const;
const route = getRouteApi("/shell/projects/home");

function QuickLinks({ slug }: { slug: string }) {
  const links = usePmQuickLinks(slug);
  const m = useQuickLinkMutations(slug);
  const [draft, setDraft] = useState({ title: "", url: "" });
  const [failed, setFailed] = useState<string | null>(null);
  const error = m.add.error ?? m.remove.error ?? (failed ? new Error(failed) : null);
  return (
    <section className={styles.widget} aria-label="Quick links">
      <h2 className={styles.h3}>Quick links</h2>
      <div className={styles.quickLinks}>
        {(links.data ?? []).map((l) => (
          <span key={l.id} className={styles.quickLink}>
            <button type="button" className={styles.rowTitle} title={l.url} onClick={() => openLink(l.url).then(() => setFailed(null), (e: unknown) => setFailed(`Couldn't open it: ${String(e)}`))}>
              🔗 {l.title || l.url}
            </button>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove ${l.title || l.url}`} onClick={() => m.remove.mutate(l.id)}>
              ×
            </button>
          </span>
        ))}
        {!links.data?.length &&
          [0, 1].map((n) => (
            <span key={n} className={`${styles.quickLink} ghostCard`}>
              {n === 0 && <span className="ghostText">{links.isError ? "Couldn't be read" : links.data ? "No quick links yet" : "Reading…"}</span>}
            </span>
          ))}
      </div>
      <form
        className={styles.inline}
        onSubmit={(e) => {
          e.preventDefault();
          if (draft.url.trim()) m.add.mutate(draft, { onSuccess: () => setDraft({ title: "", url: "" }) });
        }}
      >
        <input className={`input ${styles.search}`} aria-label="Quick link address" placeholder="example.com/handbook" value={draft.url} onChange={(e) => setDraft({ ...draft, url: e.target.value })} />
        <input className={`input ${styles.search}`} aria-label="Quick link title" placeholder="Title" value={draft.title} onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
        <button type="submit" className="btn btnSecondary btnCompact" disabled={!draft.url.trim() || m.add.isPending}>
          Add link
        </button>
      </form>
      {error && <p className="formError">{error.message}</p>}
    </section>
  );
}

function Recents({ slug }: { slug: string }) {
  const navigate = useNavigate();
  // The tab in the address (URL state).
  const search = route.useSearch();
  const tab = RECENT_TABS.find((t) => t.key === search.rtab)?.key ?? "all";
  const setTab = (key: RecentKind | "all") => void navigate({ to: "/projects/home", search: key === "all" ? {} : { rtab: key }, replace: true });
  const recents = usePmRecents(slug, tab === "all" ? null : tab);
  const open = (r: PmRecent) =>
    void (r.kind === "item"
      ? navigate({ to: "/projects/$projectId", params: { projectId: r.project_id }, search: { tab: "items", peek: r.ref_id } })
      : r.kind === "page"
        ? navigate({ to: "/projects/$projectId", params: { projectId: r.project_id }, search: { tab: "pages", page: r.ref_id } })
        : navigate({ to: "/projects/$projectId", params: { projectId: r.project_id }, search: {} }));
  const list = recents.data ?? [];
  return (
    <section className={styles.widget} aria-label="Recents">
      <div className={styles.subsHead}>
        <h2 className={styles.h3}>Recents</h2>
        <span className={styles.spacer} />
        <div className="segmented" role="tablist" aria-label="Recents">
          {RECENT_TABS.map((t) => (
            <button key={t.key} type="button" role="tab" aria-selected={tab === t.key} onClick={() => setTab(t.key)}>
              {t.label}
            </button>
          ))}
        </div>
      </div>
      {list.map((r) => (
        <button key={`${r.kind}:${r.ref_id}`} type="button" className={styles.subRow} onClick={() => open(r)}>
          <span>{r.kind === "item" ? "▢" : r.kind === "page" ? "📄" : "▦"}</span> {r.label}
          <span className="fieldHint">
            {r.kind === "project" ? "" : `${r.project_name} · `}
            {formatDateTime(r.visited_at)}
          </span>
        </button>
      ))}
      {!list.length &&
        [0, 1].map((n) => (
          <div key={n} className={`${styles.ghostRow} ghostCard`}>
            {n === 0 && <span className="ghostText">{recents.isError ? "Couldn't be read" : recents.data ? "What you open shows here" : "Reading…"}</span>}
          </div>
        ))}
    </section>
  );
}

function Stickies({ slug }: { slug: string }) {
  const stickies = usePmStickies(slug);
  const m = useStickyMutations(slug);
  const list = stickies.data ?? [];
  const error = m.add.error ?? m.update.error ?? m.remove.error;
  return (
    <section className={styles.widget} aria-label="Your stickies">
      <div className={styles.subsHead}>
        <h2 className={styles.h3}>Your stickies</h2>
        <span className={styles.spacer} />
        <button type="button" className="btn btnSecondary btnCompact" disabled={m.add.isPending} onClick={() => m.add.mutate("yellow")}>
          Add sticky
        </button>
      </div>
      {error && <p className="formError">{error.message}</p>}
      <div className={styles.stickies}>
        {list.map((s) => (
          <article key={s.id} className={styles.sticky} data-color={s.color} aria-label="Sticky">
            {/* No pictures in a sticky (pages and work items keep them). */}
            <Editor value={s.description ?? null} placeholder="Write a note…" home={{ slug, object: "pm-stickies", recordId: s.id, files: null }} onChange={(json) => m.update.mutate({ id: s.id, description: json })} />
            <div className={styles.inline}>
              <select className="input" aria-label="Colour" value={s.color} onChange={(e) => m.update.mutate({ id: s.id, color: e.target.value })}>
                {STICKY_COLORS.map((c) => (
                  <option key={c} value={c}>
                    {c.charAt(0).toUpperCase() + c.slice(1)}
                  </option>
                ))}
              </select>
              <span className={styles.spacer} />
              <DangerButton confirm="Delete this sticky? Its words go with it." onConfirm={() => m.remove.mutate(s.id)}>
                Delete
              </DangerButton>
            </div>
          </article>
        ))}
        {!list.length &&
          [0, 1, 2].map((n) => (
            <div key={n} className={`${styles.sticky} ghostCard`}>
              {n === 0 && <span className="ghostText">{stickies.isError ? "Couldn't be read" : stickies.data ? "No stickies yet" : "Reading…"}</span>}
            </div>
          ))}
      </div>
    </section>
  );
}

/** Manage widgets: each on or off, and up or down. */
function ManageWidgets({ slug, widgets }: { slug: string; widgets: HomeWidget[] }) {
  const set = useSetPmHomeWidgets(slug);
  const move = (n: number, by: number) => {
    const next = [...widgets];
    const [w] = next.splice(n, 1);
    next.splice(n + by, 0, w);
    set.mutate(next);
  };
  return (
    <details className={styles.display}>
      <summary className="btn btnGhost btnCompact">Manage widgets</summary>
      <div className={`glass ${styles.displayPanel}`}>
        {widgets.map((w, n) => (
          <div key={w.key} className={styles.inline}>
            <OnOff label={WIDGET_LABEL[w.key] ?? w.key} value={w.is_enabled} disabled={set.isPending} onChange={(on) => set.mutate(widgets.map((x) => (x.key === w.key ? { ...x, is_enabled: on } : x)))} />
            <span className={styles.spacer} />
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Move ${WIDGET_LABEL[w.key]} up`} disabled={n === 0 || set.isPending} onClick={() => move(n, -1)}>
              ↑
            </button>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Move ${WIDGET_LABEL[w.key]} down`} disabled={n === widgets.length - 1 || set.isPending} onClick={() => move(n, 1)}>
              ↓
            </button>
          </div>
        ))}
      </div>
    </details>
  );
}

/** Plane's Home (D-082): a greeting with the day and the time, then the widgets in their order —
 *  Quick links, Recents (All, Work items, Pages, Projects), Your stickies — each on or off. */
export default function HomePage() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const profile = useProfile().data;
  const widgetsQuery = usePmHomeWidgets(slug);
  const widgets = widgetsQuery.data ?? [];
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = window.setInterval(() => setNow(new Date()), 30_000);
    return () => window.clearInterval(t);
  }, []);
  const hour = now.getHours();
  const part = hour < 12 ? "morning" : hour < 18 ? "afternoon" : "evening";
  // The whole name: a Vietnamese name's first word is the family's.
  const first = profile?.name?.trim();
  return (
    <div className="page">
      <p className="eyebrow">Projects</p>
      <div className={styles.toolbar}>
        <h1 className="pageTitle">
          Good {part}
          {first ? `, ${first}` : ""}
        </h1>
        <span className={styles.spacer} />
        {slug && <ManageWidgets slug={slug} widgets={widgets} />}
      </div>
      <p className="sectionSubtitle">
        {now.toLocaleDateString(undefined, { weekday: "long" })}, {formatDay(now)} · {formatTime(now)}
      </p>
      {slug &&
        widgets
          .filter((w) => w.is_enabled)
          .map((w) => (w.key === "quick_links" ? <QuickLinks key={w.key} slug={slug} /> : w.key === "recents" ? <Recents key={w.key} slug={slug} /> : <Stickies key={w.key} slug={slug} />))}
      {widgetsQuery.data && !widgets.some((w) => w.is_enabled) &&
        [0, 1].map((n) => (
          <div key={n} className={`${styles.ghostRow} ghostCard`}>
            {n === 0 && <span className="ghostText">Every widget is off — turn one on from Manage widgets</span>}
          </div>
        ))}
    </div>
  );
}
