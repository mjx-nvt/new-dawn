import { Link } from "@tanstack/react-router";
import { useState, type ReactNode } from "react";
import type { PmProject } from "../../../kernel/bridge/pm";
import { usePmViews, useViewMutations, type PmView } from "../../../kernel/bridge/pmViews";
import { formatDateTime } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { DEFAULT_DISPLAY, LAYOUTS } from "./items";
import ViewSheet from "./ViewSheet";
import styles from "./Items.module.css";

/** One view's row: its name (opens it), description, layout and filters; ★ for the sidebar's
 *  Favorites; Lock; ⋯ Duplicate, Delete. `open` says where it opens; `readOnly` (an archived
 *  project's) keeps it as it is; `onDeleted` follows a delete. */
export function ViewRow({ slug, view: v, open, readOnly = false, onDeleted }: { slug: string; view: PmView; open: ReactNode; readOnly?: boolean; onDeleted?: () => void }) {
  const m = useViewMutations(slug);
  const [renaming, setRenaming] = useState(false);
  const busy = readOnly || m.update.isPending || m.duplicate.isPending || m.remove.isPending || m.favorite.isPending;
  return (
    <div className={styles.row}>
      <button
        type="button"
        className={`${styles.star} ${v.favorite ? styles.starOn : ""}`}
        aria-pressed={v.favorite}
        aria-label={v.favorite ? `Take ${v.name} out of Favorites` : `Add ${v.name} to Favorites`}
        disabled={busy}
        onClick={() => m.favorite.mutate({ id: v.id, on: !v.favorite })}
      >
        {v.favorite ? "★" : "☆"}
      </button>
      {renaming ? (
        <input
          className="input"
          autoFocus
          aria-label="View name"
          defaultValue={v.name}
          maxLength={255}
          onBlur={(e) => {
            setRenaming(false);
            if (e.target.value.trim() && e.target.value !== v.name) m.update.mutate({ id: v.id, patch: { name: e.target.value } });
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.nativeEvent.isComposing) e.currentTarget.blur();
            if (e.key === "Escape") setRenaming(false);
          }}
        />
      ) : (
        open
      )}
      <span className={styles.rowProps}>
        {v.description && <span className="fieldHint">{v.description}</span>}
        <span className="chip">{LAYOUTS.find((l) => l.key === v.display.layout)?.label ?? v.display.layout}</span>
        <span className="chip">{v.rules.length ? `${v.rules.length} filter${v.rules.length === 1 ? "" : "s"}` : "No filters"}</span>
        {v.is_private && <span className="chip">Private</span>}
        {v.is_locked && <span className="chip">Locked</span>}
        <span className="fieldHint">{formatDateTime(v.updated_at)}</span>
        {!readOnly && (
          <MenuButton
            label={`${v.name} actions`}
            items={[
              { label: "Rename", disabled: v.is_locked, onSelect: () => setRenaming(true) },
              { label: v.is_locked ? "Unlock" : "Lock", onSelect: () => m.update.mutate({ id: v.id, patch: { is_locked: !v.is_locked } }) },
              { label: v.is_private ? "Make it public" : "Make it private", disabled: v.is_locked, onSelect: () => m.update.mutate({ id: v.id, patch: { is_private: !v.is_private } }) },
              { label: "Duplicate", separator: true, onSelect: () => m.duplicate.mutate(v.id) },
            ]}
          />
        )}
        {!readOnly && (
          <DangerButton confirm={`Delete “${v.name}”?`} disabled={busy} onConfirm={() => m.remove.mutate(v.id, { onSuccess: onDeleted })}>
            Delete
          </DangerButton>
        )}
      </span>
      {(m.update.error ?? m.duplicate.error ?? m.remove.error ?? m.favorite.error) && <span className="formError">{(m.update.error ?? m.duplicate.error ?? m.remove.error ?? m.favorite.error)?.message}</span>}
    </div>
  );
}

/** A project's Views tab (D-077, Plane's): its saved views — each opens in Work items with its
 *  filters, layout and display; New view saves the plain list's, to change from there. */
export default function Views({ slug, project }: { slug: string; project: PmProject }) {
  const views = usePmViews(slug, project.id);
  const [making, setMaking] = useState(false);
  const list = views.data ?? [];
  return (
    <section className="section" aria-label="Views">
      <div className={styles.toolbar}>
        <div>
          <h2 className="sectionTitle">Views</h2>
          <p className="sectionSubtitle">Saved filters with a layout — open one to see its work items, change it, save it again.</p>
        </div>
        <span className={styles.spacer} />
        <button type="button" className="btn btnPrimary" disabled={!!project.archived_at} onClick={() => setMaking(true)}>
          New view
        </button>
      </div>
      {views.error && <p className="formError">{views.error.message}</p>}
      <div className={styles.group}>
        {list.map((v) => (
          <ViewRow
            key={v.id}
            slug={slug}
            view={v}
            readOnly={!!project.archived_at}
            open={
              <Link to="/projects/$projectId" params={{ projectId: project.id }} search={{ tab: "items", view: v.id }} className={styles.rowTitle}>
                {v.name}
              </Link>
            }
          />
        ))}
        {!list.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.row} ghostCard`}>
              {n === 0 && <span className="ghostText">{views.isError ? "Couldn't be read" : views.data ? "No views yet — filter the work items, then Save as a view" : "Reading…"}</span>}
            </div>
          ))}
      </div>
      {making && <ViewSheet slug={slug} projectId={project.id} rules={[]} display={DEFAULT_DISPLAY} onClose={() => setMaking(false)} onSaved={() => setMaking(false)} />}
    </section>
  );
}
