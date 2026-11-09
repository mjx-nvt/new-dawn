import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { usePeople } from "../../kernel/bridge/people";
import { readRules, usePmQuery, usePmViews, useViewMutations, type PmRule } from "../../kernel/bridge/pmViews";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import FilterBar from "./items/FilterBar";
import ItemPeek, { type PeekMode } from "./items/ItemPeek";
import { DEFAULT_DISPLAY, type Property } from "./items/items";
import Table from "./items/Table";
import ViewSheet from "./items/ViewSheet";
import { ViewRow } from "./items/Views";
import { whoName } from "./pm";
import styles from "./items/Items.module.css";
import PageHeader from "../../kernel/shell/PageHeader";

const route = getRouteApi("/shell/projects/views");

/** Plane's built-in workspace views. */
const BUILT_IN: { key: string; label: string; rules: PmRule[] }[] = [
  { key: "all", label: "All work items", rules: [] },
  { key: "assigned", label: "Assigned to me", rules: [{ field: "assignees", op: "in", values: ["me"] }] },
  { key: "created", label: "Created by me", rules: [{ field: "created_by", op: "in", values: ["me"] }] },
  { key: "subscribed", label: "Subscribed", rules: [{ field: "subscribers", op: "in", values: ["me"] }] },
];

/** The workspace's views (D-077, Plane's): All work items, Assigned to me, Created by me and
 *  Subscribed, then the views saved here — each a spreadsheet of the work items of every open
 *  project, as its filters say; the filters change (Save view keeps them on a saved one, Save as
 *  a new view makes one); ★ puts one in the sidebar's Favorites. The view, the filters and the
 *  peek are URL state. */
export default function WorkspaceViews() {
  const search = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const views = usePmViews(slug, null);
  const m = useViewMutations(slug);
  const people = usePeople(slug ?? "").data ?? [];
  const [saving, setSaving] = useState(false);
  const saved = search.view ? views.data?.find((v) => v.id === search.view) : undefined;
  const builtIn = saved ? undefined : (BUILT_IN.find((b) => b.key === (search.view ?? "all")) ?? BUILT_IN[0]);
  const rules: PmRule[] = readRules(search.f) ?? saved?.rules ?? builtIn?.rules ?? [];
  // Not before a saved view is read: its filters, not every item.
  const items = usePmQuery(slug, null, rules, !search.view || BUILT_IN.some((b) => b.key === search.view) || views.isFetched);
  const gone = !!search.view && !BUILT_IN.some((b) => b.key === search.view) && views.isFetched && !saved;
  const display = saved?.display ?? DEFAULT_DISPLAY;
  const has = (p: Property) => display.properties.includes(p);
  const who = (w: string) => (w === "automation" ? "Automation" : whoName(w, people) || w);
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/views", search: { ...search, ...patch }, replace: true });
  const changed = !!saved && JSON.stringify(rules) !== JSON.stringify(saved.rules);
  const list = items.data ?? [];
  const current = saved?.id ?? builtIn?.key;

  return (
    <div className="page">
      <PageHeader eyebrow="Projects" title="Views" meta="The work items of every open project, as each view's filters say — open a saved one here or from Favorites." />
      <div className={styles.viewsSplit}>
        <nav className={styles.viewList} aria-label="Workspace views">
          {BUILT_IN.map((b) => (
            <button key={b.key} type="button" className={styles.viewLink} aria-current={current === b.key} onClick={() => set({ view: b.key === "all" ? undefined : b.key, f: undefined, peek: undefined })}>
              {b.label}
            </button>
          ))}
          <p className="fieldLabel">Saved</p>
          {(views.data ?? []).map((v) => (
            <button key={v.id} type="button" className={styles.viewLink} aria-current={current === v.id} onClick={() => set({ view: v.id, f: undefined, peek: undefined })}>
              {v.favorite ? "★ " : ""}
              {v.name}
            </button>
          ))}
          {!views.data?.length &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{views.isError ? "Couldn't be read" : views.data ? "None saved yet" : "Reading…"}</span>}
              </div>
            ))}
        </nav>
        <section className={styles.list} aria-label={saved?.name ?? builtIn?.label ?? "Work items"}>
          {saved && slug && <ViewRow slug={slug} view={saved} open={<strong className={styles.rowTitle}>{saved.name}</strong>} onDeleted={() => set({ view: undefined, f: undefined })} />}
          {gone && <p className="formError">That view is gone — showing All work items.</p>}
          <div className={styles.inline}>
            {slug && <FilterBar slug={slug} projectId={null} rules={rules} onChange={(next) => set({ f: next.length ? JSON.stringify(next) : "[]" })} />}
            {saved && (
              <button
                type="button"
                className="btn btnSecondary btnCompact"
                title={saved.is_locked ? "Locked — unlock it to change it" : undefined}
                disabled={!changed || saved.is_locked || m.update.isPending}
                onClick={() => m.update.mutate({ id: saved.id, patch: { rules } }, { onSuccess: () => set({ f: undefined }) })}
              >
                Save view
              </button>
            )}
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setSaving(true)}>
              Save as a new view
            </button>
          </div>
          {(items.error ?? m.update.error) && <p className="formError">{(items.error ?? m.update.error)?.message}</p>}
          {slug && items.data && <Table slug={slug} items={list} labels={[]} states={[]} display={display} has={has} who={who} onPeek={(id) => set({ peek: id })} onCreate={() => undefined} groups={[{ key: "all", label: "", value: {}, items: list }]} />}
          {!items.data &&
            [0, 1].map((n) => (
              <div key={n} className={`${styles.row} ghostCard`}>
                {n === 0 && <span className="ghostText">{items.isError ? "Couldn't be read" : "Reading…"}</span>}
              </div>
            ))}
        </section>
      </div>
      {saving && slug && (
        <ViewSheet
          slug={slug}
          projectId={null}
          rules={rules}
          display={{ ...display, layout: "table" }}
          onClose={() => setSaving(false)}
          onSaved={(v) => {
            setSaving(false);
            set({ view: v.id, f: undefined });
          }}
        />
      )}
      {search.peek && slug && (
        <ItemPeek
          slug={slug}
          itemId={search.peek}
          mode={(["side", "modal", "full"] as PeekMode[]).find((x) => x === search.mode) ?? "side"}
          tab={search.activity}
          onTab={(activity) => set({ activity })}
          onMode={(x) => set({ mode: x === "side" ? undefined : x })}
          onOpen={(id) => set({ peek: id, activity: undefined })}
          onClose={() => set({ peek: undefined, mode: undefined, activity: undefined })}
        />
      )}
    </div>
  );
}
