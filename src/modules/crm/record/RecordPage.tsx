import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useNeighbours, useRecordLayout } from "../../../kernel/bridge/layouts";
import { asRecord, recordLabel, useDeleteRecord, useDestroyRecord, useObject, useRecord, useRestoreRecord, useUpdateRecord } from "../../../kernel/bridge/records";
import { useCurrentWorkspace } from "../../../kernel/bridge/workspaces";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { CellEditor } from "../cells";
import LayoutEditor from "./LayoutEditor";
import styles from "./Record.module.css";
import { isDeleted, WidgetBody, WidgetCard } from "./widgets";

const route = getRouteApi("/shell/crm/$object/$recordId");

/** Twenty's record page (D-062): the label edited in place, ‹ › through the list, ⋯ with the
 *  layout editor and Delete; the layout's tabs, each a grid of widgets. A deleted record shows
 *  a banner with Restore and Delete forever. The tab is URL state. */
export default function RecordPage() {
  const { object: objectName, recordId: id } = route.useParams();
  const { tab } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const object = useObject(slug, objectName);
  const def = object.data;
  const name = def?.meta.name_singular;
  const record = useRecord(slug, name, id);
  const layout = useRecordLayout(slug, name);
  const neighbours = useNeighbours(slug, name, id);
  const update = useUpdateRecord(slug, name);
  const remove = useDeleteRecord(slug, name);
  const restore = useRestoreRecord(slug, name);
  const destroy = useDestroyRecord(slug, name);
  const [renaming, setRenaming] = useState(false);
  const [editingLayout, setEditingLayout] = useState(false);
  const data = record.data ? asRecord(record.data) : null;
  const deleted = !!data && isDeleted(data);
  const label = def && data ? recordLabel(def, data) : "";
  const labelField = def?.fields.find((f) => f.id === def.meta.label_identifier_field_id);
  const tabs = layout.data?.tabs ?? [];
  const current = tabs.find((t) => t.tab.id === tab) ?? tabs.find((t) => t.tab.is_default) ?? tabs[0];
  const plural = def?.meta.name_plural ?? objectName;
  const showTab = (next: string) => void navigate({ to: "/crm/$object/$recordId", params: { object: objectName, recordId: id }, search: { tab: next }, replace: true });
  const go = (target: string | null | undefined) => {
    if (target) void navigate({ to: "/crm/$object/$recordId", params: { object: objectName, recordId: target }, search: tab ? { tab } : {} });
  };
  const error = object.error ?? record.error ?? layout.error ?? update.error ?? remove.error ?? restore.error ?? destroy.error;
  const ctx = def && data && slug ? { slug, object: def, record: data } : null;

  return (
    <div className="page">
      <div className={styles.header}>
        <div className={styles.titleBlock}>
          <p className="eyebrow">{def?.meta.label_singular ?? "Record"}</p>
          {renaming && ctx && labelField ? (
            <div className={styles.titleEditor}>
              <CellEditor
                slug={ctx.slug}
                field={labelField}
                record={ctx.record}
                onSave={(v) => {
                  setRenaming(false);
                  update.mutate({ id, data: { [labelField.name]: v } });
                }}
                onCancel={() => setRenaming(false)}
              />
            </div>
          ) : (
            <h1 className="pageTitle">
              <button type="button" className={styles.titleButton} disabled={!ctx || deleted || !labelField} title="Rename" onClick={() => setRenaming(true)}>
                {ctx ? label || <span className="ghostText">Unnamed</span> : "…"}
              </button>
            </h1>
          )}
        </div>
        <div className={styles.nav}>
          <Link to="/crm" search={{ tab: plural }} className="btn btnGhost btnCompact">
            All {def?.meta.label_plural.toLowerCase() ?? "records"}
          </Link>
          <button type="button" className="btn btnSecondary btnCompact" aria-label="Previous record" disabled={!neighbours.data?.prev} onClick={() => go(neighbours.data?.prev)}>
            ‹
          </button>
          <button type="button" className="btn btnSecondary btnCompact" aria-label="Next record" disabled={!neighbours.data?.next} onClick={() => go(neighbours.data?.next)}>
            ›
          </button>
          <MenuButton
            label="More"
            items={[
              { label: "Edit page layout", onSelect: () => setEditingLayout(true), disabled: !ctx || !layout.data },
              deleted ? { label: "Restore", onSelect: () => restore.mutate(id), separator: true } : { label: "Delete", danger: true, separator: true, disabled: !ctx, onSelect: () => remove.mutate(id) },
            ]}
          />
        </div>
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {deleted && (
        <div className={styles.banner} role="status">
          <span>This {def?.meta.label_singular.toLowerCase() ?? "record"} is in Deleted records.</span>
          <button type="button" className="btn btnSecondary btnCompact" disabled={restore.isPending} onClick={() => restore.mutate(id)}>
            Restore
          </button>
          <DangerButton confirm="Delete forever?" disabled={destroy.isPending} onConfirm={() => destroy.mutate(id, { onSuccess: () => void navigate({ to: "/crm", search: { tab: plural } }) })}>
            Delete forever
          </DangerButton>
        </div>
      )}
      {editingLayout && ctx && layout.data ? (
        <LayoutEditor ctx={ctx} layout={layout.data} onDone={() => setEditingLayout(false)} />
      ) : (
        <>
          <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="Record page">
            {tabs.length ? (
              tabs.map((t) => (
                <button key={t.tab.id} type="button" role="tab" aria-selected={t.tab.id === current?.tab.id} onClick={() => showTab(t.tab.id)}>
                  {t.tab.title}
                </button>
              ))
            ) : (
              <button type="button" role="tab" aria-selected>
                Overview
              </button>
            )}
          </div>
          <div role="tabpanel" className={styles.grid}>
            {ctx && current ? (
              current.widgets.map((w) => (
                <WidgetCard key={w.id} widget={w}>
                  <WidgetBody ctx={ctx} widget={w} />
                </WidgetCard>
              ))
            ) : (
              <div className={`${styles.widgetGhost} ghostCard`}>{record.isError ? <span className="ghostText">Couldn't be read</span> : null}</div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
