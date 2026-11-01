import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useObject, useObjects, type FieldMeta, type ObjectDef } from "../../kernel/bridge/records";
import { useCreateView, useSaveView, useUpdateView, useView, useViews, type ViewDef } from "../../kernel/bridge/views";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import PeoplePage from "../people/PeoplePage";
import styles from "./Crm.module.css";
import RecordTable, { groupsOf, type Column } from "./RecordTable";
import CalendarView from "./views/CalendarView";
import KanbanBoard from "./views/KanbanBoard";
import { fromView, newRule, nextPosition, queryOf, same, useWorking, type Working } from "./views/useViewState";
import ViewBar from "./views/ViewBar";

const route = getRouteApi("/shell/crm");

/** Twenty's objects as tabs — People, Companies, Opportunities, Tasks, Notes, each with its views
 *  (D-060, D-061) — and New Dawn's Commitments (D-055) beside them. The tab and the view are
 *  URL state. */
export default function CrmPage() {
  const { tab, view } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const objects = useObjects(slug);
  const shown = (objects.data ?? []).filter((o) => !o.is_system && o.is_active);
  const current = tab === "commitments" ? undefined : (shown.find((o) => o.name_plural === tab) ?? shown[0]);
  const object = useObject(slug, current?.name_singular);
  const [search, setSearch] = useState("");
  const [deleted, setDeleted] = useState(false);
  const show = (next: string) => void navigate({ to: "/crm", search: { tab: next }, replace: true });
  const showView = (id: string) => void navigate({ to: "/crm", search: { tab, view: id }, replace: true });
  const title = tab === "commitments" ? "Commitments" : (current?.label_plural ?? "CRM");

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">{title}</h1>
        </div>
        {tab === "commitments" ? (
          <Link to="/workspace" search={{ tab: "people" }} className="btn btnGhost">
            Names and spellings
          </Link>
        ) : (
          <div className={styles.tools}>
            <input className={`input ${styles.search}`} type="search" placeholder={`Search ${title.toLowerCase()}…`} aria-label={`Search ${title.toLowerCase()}`} value={search} onChange={(e) => setSearch(e.target.value)} />
            <button type="button" className={deleted ? "btn btnSecondary btnCompact" : "btn btnGhost btnCompact"} aria-pressed={deleted} onClick={() => setDeleted(!deleted)}>
              Deleted records
            </button>
          </div>
        )}
      </div>

      <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="CRM">
        {shown.map((o) => (
          <button key={o.id} type="button" role="tab" aria-selected={tab !== "commitments" && o.id === current?.id} onClick={() => show(o.name_plural)}>
            {o.label_plural}
          </button>
        ))}
        <button type="button" role="tab" aria-selected={tab === "commitments"} onClick={() => show("commitments")}>
          Commitments
        </button>
      </div>

      <div role="tabpanel">
        {tab === "commitments" ? (
          <PeoplePage embedded />
        ) : slug && object.data ? (
          <ObjectViews key={object.data.meta.id} slug={slug} object={object.data} viewId={view} search={search} deleted={deleted} onPickView={showView} />
        ) : (
          <div className={styles.tableWrap}>
            <div className={`${styles.slot} ghostCard`}>{objects.isError || object.isError ? <span className="ghostText">Couldn't be read — {(objects.error ?? object.error)?.message}</span> : null}</div>
          </div>
        )}
      </div>
    </div>
  );
}

/** An object's view bar and table: the saved view, the working copy, and what saves it. */
function ObjectViews({ slug, object, viewId, search, deleted, onPickView }: { slug: string; object: ObjectDef; viewId: string | undefined; search: string; deleted: boolean; onPickView: (id: string) => void }) {
  const name = object.meta.name_singular;
  const views = useViews(slug, name);
  const list = views.data ?? [];
  const chosen = list.find((v) => v.id === viewId) ?? list.find((v) => v.key === "INDEX") ?? list[0];
  const def = useView(slug, chosen?.id);
  const [working, setWorking, reset] = useWorking(def.data);
  const saveView = useSaveView(slug, name);
  const createView = useCreateView(slug, name);
  const updateView = useUpdateView(slug, name);
  const [saving, setSaving] = useState<Error | null>(null);

  if (!def.data || !working) {
    return (
      <div className={styles.tableWrap}>
        <div className={`${styles.slot} ghostCard`}>{views.isError || def.isError ? <span className="ghostText">Couldn't be read — {(views.error ?? def.error)?.message}</span> : null}</div>
      </div>
    );
  }
  const current: ViewDef = def.data;
  const dirty = !same(working, fromView(current));
  const fieldOf = (id: string) => object.fields.find((f) => f.id === id);
  const columns: Column[] = working.fields.filter((f) => f.is_visible).flatMap((f) => {
    const field = fieldOf(f.field_id);
    return field ? [{ field, size: f.size, aggregate: f.aggregate_operation ?? null }] : [];
  });
  const groupField = working.groupBy ? fieldOf(working.groupBy) : undefined;
  const query = queryOf(object, working, { search, deleted });

  /** Writes the working copy into a view — fields, filters, sorts, groups, grouping — in one go:
   *  a failure changes nothing, and the copy resyncs once, to the saved view. */
  const persist = async (id: string, w: Working) => {
    await saveView.mutateAsync({ id, input: { fields: w.fields, filters: w.filters, sorts: w.sorts, groups: w.groups, group_by_field_id: w.groupBy, group_sort: w.groupSort } });
  };
  const save = () => {
    setSaving(null);
    persist(current.view.id, working).catch((e: Error) => setSaving(e));
  };
  const saveAs = (viewName: string) => {
    setSaving(null);
    createView
      .mutateAsync({ name: viewName, type: current.view.type })
      .then(async (made) => {
        if (current.view.type !== "TABLE") {
          await updateView.mutateAsync({
            id: made.view.id,
            patch: {
              kanban_field_id: current.view.kanban_field_id ?? null,
              kanban_aggregate_operation: current.view.kanban_aggregate_operation ?? null,
              kanban_aggregate_field_id: current.view.kanban_aggregate_field_id ?? null,
              calendar_field_id: current.view.calendar_field_id ?? null,
              is_compact: current.view.is_compact,
            },
          });
        }
        await persist(made.view.id, working);
        onPickView(made.view.id);
      })
      .catch((e: Error) => setSaving(e));
  };
  const change = (next: Working) => setWorking(next);
  const sort = (field: FieldMeta, direction: "ASC" | "DESC") => {
    const rest = working.sorts.filter((s) => s.field_id !== field.id);
    change({ ...working, sorts: [...rest, { field_id: field.id, sub: "", direction, position: nextPosition(rest) }] });
  };
  const filter = (field: FieldMeta) => change({ ...working, filters: { ...working.filters, filters: [...working.filters.filters, newRule(field, null, nextPosition(working.filters.filters))] } });
  const fieldsWith = (fn: (f: Working["fields"][number]) => Working["fields"][number]) => change({ ...working, fields: working.fields.map(fn) });
  const kanbanField = current.view.type === "KANBAN" ? fieldOf(current.view.kanban_field_id ?? "") : undefined;
  const calendarField = current.view.type === "CALENDAR" ? fieldOf(current.view.calendar_field_id ?? "") : undefined;
  const aggregateField = fieldOf(current.view.kanban_aggregate_field_id ?? "");
  const aggregateOp = current.view.kanban_aggregate_operation;
  const aggregate = aggregateOp ? { field: aggregateField ?? (aggregateOp === "COUNT" ? columns[0]?.field : undefined), op: aggregateOp } : null;

  return (
    <>
      <ViewBar slug={slug} object={object} views={list} current={current} working={working} dirty={dirty} onPick={onPickView} onChange={change} onSave={save} onSaveAs={saveAs} onReset={reset} />
      {saving && (
        <p className="formError" role="alert">
          {saving.message}
        </p>
      )}
      {current.view.type === "KANBAN" ? (
        kanbanField ? (
          <KanbanBoard slug={slug} object={object} field={kanbanField} columns={columns} query={query} deleted={deleted} compact={current.view.is_compact} groups={groupsOf(kanbanField, working.groups, working.groupSort)} aggregate={aggregate && aggregate.field ? { field: aggregate.field, op: aggregate.op } : null} />
        ) : (
          <div className={styles.tableWrap}>
            <div className={`${styles.slot} ghostCard`}>
              <span className="ghostText">Pick the select field for the columns in ⋯ Options → Columns.</span>
            </div>
          </div>
        )
      ) : current.view.type === "CALENDAR" ? (
        calendarField ? (
          <CalendarView slug={slug} object={object} field={calendarField} query={query} deleted={deleted} />
        ) : (
          <div className={styles.tableWrap}>
            <div className={`${styles.slot} ghostCard`}>
              <span className="ghostText">Pick the date field in ⋯ Options → Date field.</span>
            </div>
          </div>
        )
      ) : (
      <RecordTable
        slug={slug}
        object={object}
        columns={columns}
        query={query}
        deleted={deleted}
        compact={current.view.is_compact}
        groupBy={groupField}
        groups={groupField ? groupsOf(groupField, working.groups, working.groupSort) : undefined}
        onResize={(id, size) => fieldsWith((f) => (f.field_id === id ? { ...f, size } : f))}
        onReorder={(id, overId) => {
          const rest = working.fields.filter((f) => f.field_id !== id);
          const moving = working.fields.find((f) => f.field_id === id);
          if (!moving) return;
          rest.splice(
            rest.findIndex((f) => f.field_id === overId),
            0,
            moving,
          );
          change({ ...working, fields: rest.map((f, i) => ({ ...f, position: i })) });
        }}
        onSort={sort}
        onHide={(id) => fieldsWith((f) => (f.field_id === id ? { ...f, is_visible: false } : f))}
        onFilter={filter}
        onAggregate={(id, op) => fieldsWith((f) => (f.field_id === id ? { ...f, aggregate_operation: op } : f))}
      />
      )}
    </>
  );
}
