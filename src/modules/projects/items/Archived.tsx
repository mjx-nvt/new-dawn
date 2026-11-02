import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useArchivePmItem, useDeletePmItem, usePmItems } from "../../../kernel/bridge/pmItems";
import { formatDateTime } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { Dot } from "./Props";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");

/** A project's archived work items (D-074): read-only, restored or deleted here; one opens in
 *  its peek. */
export default function Archived({ slug, projectId }: { slug: string; projectId: string }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const items = usePmItems(slug, projectId, true);
  const restore = useArchivePmItem(slug);
  const remove = useDeletePmItem(slug);
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId }, search: { ...search, ...patch }, replace: true });
  const list = items.data ?? [];
  return (
    <section className="section" aria-label="Archives">
      <h2 className="sectionTitle">Archived work items</h2>
      <p className="sectionSubtitle">Completed or cancelled work items set aside — read-only, still here to read, restore or delete.</p>
      {(items.error ?? restore.error ?? remove.error) && <p className="formError">{(items.error ?? restore.error ?? remove.error)?.message}</p>}
      <div className={styles.group}>
        {list.map((i) => (
          <div key={i.id} className={styles.row}>
            <span className={styles.key}>{i.key}</span>
            <button type="button" className={styles.rowTitle} onClick={() => set({ peek: i.id })}>
              {i.name}
            </button>
            <span className={styles.rowProps}>
              <span className={styles.prop}>
                <Dot color={i.state_color} /> {i.state_name}
              </span>
              <span className="fieldHint">archived {i.archived_at ? formatDateTime(i.archived_at) : ""}</span>
              <button type="button" className="btn btnSecondary btnCompact" disabled={restore.isPending} onClick={() => restore.mutate({ id: i.id, archived: false })}>
                Restore
              </button>
              <DangerButton confirm={i.sub_items ? `Delete ${i.key} and its ${i.sub_items} sub-work item${i.sub_items === 1 ? "" : "s"}?` : `Delete ${i.key}?`} disabled={remove.isPending} onConfirm={() => remove.mutate(i.id)}>
                Delete
              </DangerButton>
            </span>
          </div>
        ))}
        {!list.length &&
          [0, 1].map((n) => (
            <div key={n} className={`${styles.row} ghostCard`}>
              {n === 0 && <span className="ghostText">{items.isError ? "Couldn't be read" : items.data ? "Nothing archived" : "Reading…"}</span>}
            </div>
          ))}
      </div>
      {search.peek && (
        <ItemPeek
          slug={slug}
          itemId={search.peek}
          mode={(["side", "modal", "full"] as PeekMode[]).find((m) => m === search.mode) ?? "side"}
          tab={search.activity}
          onTab={(activity) => set({ activity })}
          onMode={(m) => set({ mode: m === "side" ? undefined : m })}
          onOpen={(id) => set({ peek: id, activity: undefined })}
          onClose={() => set({ peek: undefined, mode: undefined, activity: undefined })}
        />
      )}
    </section>
  );
}
