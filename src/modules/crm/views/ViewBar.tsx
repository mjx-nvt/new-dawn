import { useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { useAddFavorite, useFavorites, useRemoveFavorite } from "../../../kernel/bridge/navigation";
import { selectOptions, type FieldMeta, type ObjectDef } from "../../../kernel/bridge/records";
import { useCreateView, useDeleteView, useDuplicateView, useReorderViews, useUpdateView, type ViewDef, type ViewPatch, type ViewRow } from "../../../kernel/bridge/views";
import OnOff from "../../workspace-settings/OnOff";
import { AdvancedFilter, RuleEditor, ruleText } from "./FilterEditor";
import { newRule, nextPosition, subsOf, usableFields, type Working } from "./useViewState";
import styles from "./Views.module.css";

/** Twenty's view bar (D-061): the view dropdown on the left; Filter, Sort and Options on the
 *  right; the active rules and sorts as chips underneath; Update view · Create view · Reset
 *  when the working copy differs from the saved view. */
interface Props {
  slug: string;
  object: ObjectDef;
  views: ViewRow[];
  current: ViewDef;
  working: Working;
  dirty: boolean;
  onPick: (id: string) => void;
  onChange: (next: Working) => void;
  onSave: () => void;
  onSaveAs: (name: string) => void;
  onReset: () => void;
}

export default function ViewBar({ slug, object, views, current, working, dirty, onPick, onChange, onSave, onSaveAs, onReset }: Props) {
  const [open, setOpen] = useState<"views" | "filter" | "sort" | "options" | "advanced" | "saveAs" | null>(null);
  const [editingRule, setEditingRule] = useState<number | null>(null);
  const close = () => {
    setOpen(null);
    setEditingRule(null);
  };
  const isIndex = current.view.key === "INDEX";
  const topRules = working.filters.filters.map((r, i) => ({ r, i })).filter(({ r }) => !r.group_id);
  const grouped = working.filters.filters.length - topRules.length + working.filters.groups.length;
  const fieldOf = (id: string) => object.fields.find((f) => f.id === id);

  const addRule = (field: FieldMeta) => {
    const next = { ...working, filters: { ...working.filters, filters: [...working.filters.filters, newRule(field, null, nextPosition(working.filters.filters))] } };
    onChange(next);
    setEditingRule(next.filters.filters.length - 1);
    setOpen("filter");
  };
  const removeRule = (i: number) => onChange({ ...working, filters: { ...working.filters, filters: working.filters.filters.filter((_, j) => j !== i) } });
  const addSort = (field: FieldMeta, sub: string, direction: "ASC" | "DESC") => {
    const rest = working.sorts.filter((s) => s.field_id !== field.id);
    onChange({ ...working, sorts: [...rest, { field_id: field.id, sub, direction, position: nextPosition(rest) }] });
    close();
  };

  return (
    <>
      <div className={styles.bar}>
        <div className={styles.left}>
          <Anchor open={open === "views"} onClose={close}>
            <button type="button" className={`btn btnGhost ${styles.viewButton}`} aria-haspopup="menu" aria-expanded={open === "views"} onClick={() => setOpen(open === "views" ? null : "views")}>
              {current.view.name} ▾
            </button>
            {open === "views" && <ViewsMenu slug={slug} object={object} views={views} currentId={current.view.id} onPick={(id) => (onPick(id), close())} />}
          </Anchor>
          {dirty && (
            <div className={styles.dirty}>
              <button type="button" className="btn btnGhost btnCompact" onClick={onReset}>
                Reset
              </button>
              {isIndex ? (
                <button type="button" className="btn btnSecondary btnCompact" onClick={() => setOpen("saveAs")}>
                  Save as new view
                </button>
              ) : (
                <>
                  <button type="button" className="btn btnSecondary btnCompact" onClick={onSave}>
                    Update view
                  </button>
                  <button type="button" className="btn btnGhost btnCompact" aria-label="Create view from these changes" onClick={() => setOpen("saveAs")}>
                    ▾
                  </button>
                </>
              )}
              {open === "saveAs" && (
                <Anchor open onClose={close}>
                  <NamePrompt label="New view" onDone={(name) => (onSaveAs(name), close())} onCancel={close} />
                </Anchor>
              )}
            </div>
          )}
        </div>
        <div className={styles.right}>
          <Anchor open={open === "filter"} onClose={close}>
            <button type="button" className="btn btnGhost btnCompact" aria-haspopup="menu" aria-expanded={open === "filter"} onClick={() => (setOpen(open === "filter" ? null : "filter"), setEditingRule(null))}>
              Filter
            </button>
            {open === "filter" &&
              (editingRule !== null && working.filters.filters[editingRule] ? (
                <div className={`glass ${styles.panel} ${styles.panelRight}`}>
                  <RuleEditor slug={slug} object={object} rule={working.filters.filters[editingRule]} onChange={(next) => onChange({ ...working, filters: { ...working.filters, filters: working.filters.filters.map((x, j) => (j === editingRule ? next : x)) } })} onRemove={() => (removeRule(editingRule), close())} />
                  <div className={styles.actions}>
                    <button type="button" className="btn btnSecondary btnCompact" onClick={close}>
                      Done
                    </button>
                  </div>
                </div>
              ) : (
                <div className={`glass ${styles.panel} ${styles.panelRight}`}>
                  <p className={styles.panelTitle}>Filter by</p>
                  <div className={styles.list}>
                    {usableFields(object).map((f) => (
                      <button key={f.id} type="button" className={styles.listButton} onClick={() => addRule(f)}>
                        {f.label}
                      </button>
                    ))}
                  </div>
                  <button type="button" className="btn btnGhost btnCompact" onClick={() => setOpen("advanced")}>
                    Advanced filter
                  </button>
                </div>
              ))}
            {open === "advanced" && (
              <div className={`glass ${styles.panel} ${styles.panelRight} ${styles.panelWide}`}>
                <p className={styles.panelTitle}>Advanced filter</p>
                <AdvancedFilter slug={slug} object={object} working={working} onChange={onChange} />
                <div className={styles.actions}>
                  <button type="button" className="btn btnSecondary btnCompact" onClick={close}>
                    Done
                  </button>
                </div>
              </div>
            )}
          </Anchor>
          <Anchor open={open === "sort"} onClose={close}>
            <button type="button" className="btn btnGhost btnCompact" aria-haspopup="menu" aria-expanded={open === "sort"} onClick={() => setOpen(open === "sort" ? null : "sort")}>
              Sort
            </button>
            {open === "sort" && <SortMenu object={object} onPick={addSort} />}
          </Anchor>
          <Anchor open={open === "options"} onClose={close}>
            <button type="button" className="btn btnGhost btnCompact" aria-haspopup="menu" aria-expanded={open === "options"} aria-label="Options" onClick={() => setOpen(open === "options" ? null : "options")}>
              ⋯
            </button>
            {open === "options" && <OptionsMenu slug={slug} object={object} current={current} working={working} onChange={onChange} />}
          </Anchor>
        </div>
      </div>
      {(topRules.length > 0 || grouped > 0 || working.sorts.length > 0) && (
        <div className={styles.chips}>
          {topRules.map(({ r, i }) => (
            <span key={i} className={styles.chip}>
              <button type="button" className={`${styles.chipX} ${styles.chipText}`} onClick={() => (setEditingRule(i), setOpen("filter"))}>
                {ruleText(object, r)}
              </button>
              <button type="button" className={styles.chipX} aria-label="Remove filter" onClick={() => removeRule(i)}>
                ✕
              </button>
            </span>
          ))}
          {grouped > 0 && (
            <button type="button" className={styles.chip} onClick={() => setOpen("advanced")}>
              {grouped === 1 ? "1 advanced rule" : `${grouped} advanced rules`}
            </button>
          )}
          {working.sorts.map((s) => (
            <span key={s.field_id} className={`${styles.chip} ${styles.chipNeutral}`}>
              {fieldOf(s.field_id)?.label ?? "?"}
              {s.sub && fieldOf(s.field_id) ? ` · ${subsOf(fieldOf(s.field_id) as FieldMeta).find((p) => p.key === s.sub)?.label.toLowerCase() ?? s.sub}` : ""} {s.direction === "DESC" ? "↓" : "↑"}
              <button type="button" className={styles.chipX} aria-label="Remove sort" onClick={() => onChange({ ...working, sorts: working.sorts.filter((x) => x !== s) })}>
                ✕
              </button>
            </span>
          ))}
        </div>
      )}
    </>
  );
}

/** A button and the panel it opens; a click outside or Escape closes it. */
function Anchor({ open, onClose, children }: { open: boolean; onClose: () => void; children: ReactNode }) {
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      if (root.current && !root.current.contains(e.target as Node)) onClose();
    };
    const key = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    document.addEventListener("mousedown", down);
    document.addEventListener("keydown", key);
    return () => {
      document.removeEventListener("mousedown", down);
      document.removeEventListener("keydown", key);
    };
  }, [open, onClose]);
  return (
    <div ref={root} className={styles.anchor}>
      {children}
    </div>
  );
}

/** "+ Add view": a name, the type — Table, Kanban (a select field for its columns), Calendar (a
 *  date field) — as Twenty asks. */
function NewViewPrompt({ object, onDone, onCancel }: { object: ObjectDef; onDone: (name: string, type: string, fieldId: string | null) => void; onCancel: () => void }) {
  const [name, setName] = useState("");
  const [type, setType] = useState<"TABLE" | "KANBAN" | "CALENDAR">("TABLE");
  const [fieldId, setFieldId] = useState("");
  const selects = object.fields.filter((f) => f.is_active && f.type === "SELECT");
  const dates = object.fields.filter((f) => f.is_active && (f.type === "DATE" || f.type === "DATE_TIME"));
  const needs = type === "KANBAN" ? selects : type === "CALENDAR" ? dates : [];
  const chosen = needs.some((f) => f.id === fieldId) ? fieldId : (needs[0]?.id ?? "");
  const ok = !!name.trim() && (type === "TABLE" || !!chosen);
  const done = () => ok && onDone(name.trim(), type, type === "TABLE" ? null : chosen);
  return (
    <div className={`glass ${styles.panel}`}>
      <p className={styles.panelTitle}>New view</p>
      <input
        className={`input ${styles.nameInput}`}
        autoFocus
        aria-label="New view"
        value={name}
        placeholder="Name"
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) done();
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className="segmented" role="group" aria-label="View type">
        {(["TABLE", "KANBAN", "CALENDAR"] as const).map((t) => (
          <button key={t} type="button" aria-pressed={type === t} onClick={() => setType(t)}>
            {t === "TABLE" ? "Table" : t === "KANBAN" ? "Kanban" : "Calendar"}
          </button>
        ))}
      </div>
      {type !== "TABLE" &&
        (needs.length ? (
          <select className="input" aria-label={type === "KANBAN" ? "Columns" : "Date field"} value={chosen} onChange={(e) => setFieldId(e.target.value)}>
            {needs.map((f) => (
              <option key={f.id} value={f.id}>
                {f.label}
              </option>
            ))}
          </select>
        ) : (
          <p className="fieldHint">{type === "KANBAN" ? "A kanban needs a select field on this object." : "A calendar needs a date field on this object."}</p>
        ))}
      <div className={styles.actions}>
        <button type="button" className="btn btnPrimary btnCompact" disabled={!ok} onClick={done}>
          Create
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}

function NamePrompt({ label, initial = "", onDone, onCancel }: { label: string; initial?: string; onDone: (name: string) => void; onCancel: () => void }) {
  const [name, setName] = useState(initial);
  return (
    <div className={`glass ${styles.panel}`}>
      <p className={styles.panelTitle}>{label}</p>
      <input
        className={`input ${styles.nameInput}`}
        autoFocus
        aria-label={label}
        value={name}
        placeholder="Name"
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing && name.trim()) onDone(name.trim());
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className={styles.actions}>
        <button type="button" className="btn btnPrimary btnCompact" disabled={!name.trim()} onClick={() => onDone(name.trim())}>
          Save
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={onCancel}>
          Cancel
        </button>
      </div>
    </div>
  );
}

/** The views of the object: pick one, drag to reorder, ⋮ to rename, duplicate or delete, add. */
function ViewsMenu({ slug, object, views, currentId, onPick }: { slug: string; object: ObjectDef; views: ViewRow[]; currentId: string; onPick: (id: string) => void }) {
  const name = object.meta.name_singular;
  const create = useCreateView(slug, name);
  const update = useUpdateView(slug, name);
  const remove = useDeleteView(slug, name);
  const duplicate = useDuplicateView(slug, name);
  const reorder = useReorderViews(slug, name);
  // A view pinned in the sidebar's Favorites (D-064): the favorite's id by view.
  const favorites = useFavorites(slug);
  const addFavorite = useAddFavorite(slug);
  const removeFavorite = useRemoveFavorite(slug);
  const pins = new Map((favorites.data?.items ?? []).flatMap((f) => (f.view_id ? [[f.view_id, f.id] as const] : [])));
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [menu, setMenu] = useState<string | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const drop = (overId: string) => {
    if (!dragging || dragging === overId) return;
    const ids = views.map((v) => v.id).filter((id) => id !== dragging);
    ids.splice(ids.indexOf(overId), 0, dragging);
    reorder.mutate(ids);
    setDragging(null);
  };
  const error = create.error ?? update.error ?? remove.error ?? duplicate.error;
  return (
    <div className={`glass ${styles.panel}`}>
      <p className={styles.panelTitle}>Views</p>
      <div className={styles.list}>
        {views.map((v) =>
          renaming === v.id ? (
            <NamePrompt key={v.id} label="Rename view" initial={v.name} onDone={(n) => (update.mutate({ id: v.id, patch: { name: n } }), setRenaming(null))} onCancel={() => setRenaming(null)} />
          ) : (
            <div key={v.id} className={`${styles.listItem} ${dragging === v.id ? styles.dragging : ""}`} draggable onDragStart={() => setDragging(v.id)} onDragOver={(e) => e.preventDefault()} onDrop={() => drop(v.id)} onDragEnd={() => setDragging(null)}>
              <button type="button" className={`${styles.listButton} ${v.id === currentId ? styles.current : ""}`} onClick={() => onPick(v.id)}>
                {v.name}
                {v.visibility === "UNLISTED" && <span className={styles.muted}> · unlisted</span>}
              </button>
              <button type="button" className={styles.iconButton} aria-label={`Actions for ${v.name}`} onClick={() => setMenu(menu === v.id ? null : v.id)}>
                ⋮
              </button>
              {menu === v.id && (
                <div className={`glass menu ${styles.rowMenu}`} role="menu">
                  <button type="button" role="menuitem" className="menuItem" onClick={() => (setRenaming(v.id), setMenu(null))}>
                    Edit
                  </button>
                  <button type="button" role="menuitem" className="menuItem" onClick={() => (duplicate.mutate({ id: v.id }), setMenu(null))}>
                    Duplicate
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    className="menuItem"
                    onClick={() => {
                      const pinned = pins.get(v.id);
                      if (pinned) removeFavorite.mutate(pinned);
                      else addFavorite.mutate({ object: null, record_id: null, view_id: v.id });
                      setMenu(null);
                    }}
                  >
                    {pins.has(v.id) ? "Remove from favorites" : "Add to favorites"}
                  </button>
                  {v.key !== "INDEX" && (
                    <button type="button" role="menuitem" className="menuItem menuItemDanger" onClick={() => (remove.mutate(v.id), setMenu(null), v.id === currentId && onPick(views[0]?.id ?? v.id))}>
                      Delete
                    </button>
                  )}
                </div>
              )}
            </div>
          ),
        )}
      </div>
      {adding ? (
        <NewViewPrompt
          object={object}
          onDone={(n, t, fieldId) => {
            create.mutate(
              { name: n, type: t },
              {
                onSuccess: (def) => {
                  if (!fieldId) return onPick(def.view.id);
                  update.mutate({ id: def.view.id, patch: t === "KANBAN" ? { kanban_field_id: fieldId } : { calendar_field_id: fieldId } }, { onSuccess: () => onPick(def.view.id) });
                },
              },
            );
            setAdding(false);
          }}
          onCancel={() => setAdding(false)}
        />
      ) : (
        <button type="button" className="btn btnGhost btnCompact" onClick={() => setAdding(true)}>
          + Add view
        </button>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
    </div>
  );
}

function SortMenu({ object, onPick }: { object: ObjectDef; onPick: (field: FieldMeta, sub: string, direction: "ASC" | "DESC") => void }) {
  const [direction, setDirection] = useState<"ASC" | "DESC">("ASC");
  return (
    <div className={`glass ${styles.panel} ${styles.panelRight}`}>
      <div className="segmented" role="group" aria-label="Direction">
        <button type="button" aria-pressed={direction === "ASC"} onClick={() => setDirection("ASC")}>
          Ascending
        </button>
        <button type="button" aria-pressed={direction === "DESC"} onClick={() => setDirection("DESC")}>
          Descending
        </button>
      </div>
      <div className={styles.list}>
        {usableFields(object).flatMap((f) => {
          const parts = subsOf(f);
          const entries = parts.length > 1 ? parts.map((p) => ({ sub: p.key, label: `${f.label} · ${p.label.toLowerCase()}` })) : [{ sub: "", label: f.label }];
          return entries.map((e) => (
            <button key={`${f.id}-${e.sub}`} type="button" className={styles.listButton} onClick={() => onPick(f, e.sub, direction)}>
              {e.label}
            </button>
          ));
        })}
      </div>
    </div>
  );
}

/** Options: Fields (show, hide, reorder), Group (a select field, its order, hidden groups),
 *  Compact view, Open record in, Visibility. */
function OptionsMenu({ slug, object, current, working, onChange }: { slug: string; object: ObjectDef; current: ViewDef; working: Working; onChange: (next: Working) => void }) {
  const update = useUpdateView(slug, object.meta.name_singular);
  const [section, setSection] = useState<"fields" | "group" | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const fieldOf = (id: string) => object.fields.find((f) => f.id === id);
  const setFields = (fields: Working["fields"]) => onChange({ ...working, fields: fields.map((f, i) => ({ ...f, position: i })) });
  const move = (id: string, overId: string) => {
    if (id === overId) return;
    const rest = working.fields.filter((f) => f.field_id !== id);
    const moving = working.fields.find((f) => f.field_id === id);
    if (!moving) return;
    rest.splice(
      rest.findIndex((f) => f.field_id === overId),
      0,
      moving,
    );
    setFields(rest);
  };
  const kanban = current.view.type === "KANBAN";
  const calendar = current.view.type === "CALENDAR";
  const groupField = kanban ? fieldOf(current.view.kanban_field_id ?? "") : working.groupBy ? fieldOf(working.groupBy) : undefined;
  const groupOptions = groupField ? selectOptions(groupField) : [];
  const groupsInOrder = [...groupOptions.map((o) => o.value), ""].map((value) => working.groups.find((g) => g.field_value === value) ?? { field_value: value, is_visible: true, position: working.groups.length + groupOptions.findIndex((o) => o.value === value) });
  const sortedGroups = working.groupSort === "MANUAL" ? [...groupsInOrder].sort((a, b) => a.position - b.position) : groupsInOrder;
  const setGroups = (groups: Working["groups"]) => onChange({ ...working, groups: groups.map((g, i) => ({ ...g, position: i })) });
  const labelOfGroup = (value: string) => groupOptions.find((o) => o.value === value)?.label ?? "No value";
  const selects = object.fields.filter((f) => f.is_active && f.type === "SELECT");
  const dates = object.fields.filter((f) => f.is_active && (f.type === "DATE" || f.type === "DATE_TIME"));
  const numbers = object.fields.filter((f) => f.is_active && ["NUMBER", "CURRENCY", "RATING"].includes(f.type));
  const patchNow = (p: ViewPatch) => update.mutate({ id: current.view.id, patch: p });
  // Import records / Export view (D-065): both are the page's, reached by URL state.
  const navigate = useNavigate();
  const { q } = useSearch({ from: "/shell/crm" });
  const transfer = (flag: "import" | "export") => void navigate({ to: "/crm", search: { tab: object.meta.name_plural, view: current.view.id, ...(q ? { q } : {}), [flag]: true } });

  if (section === "fields") {
    return (
      <div className={`glass ${styles.panel} ${styles.panelRight}`}>
        <div className={styles.row}>
          <button type="button" className={styles.iconButton} aria-label="Back" onClick={() => setSection(null)}>
            ‹
          </button>
          <p className={styles.panelTitle}>Fields</p>
        </div>
        <div className={styles.list}>
          {working.fields.map((f, i) => {
            const meta = fieldOf(f.field_id);
            if (!meta) return null;
            const isLabel = meta.id === object.meta.label_identifier_field_id;
            return (
              <div key={f.field_id} className={`${styles.listItem} ${dragging === f.field_id ? styles.dragging : ""}`} draggable={!isLabel} onDragStart={() => setDragging(f.field_id)} onDragOver={(e) => e.preventDefault()} onDrop={() => dragging && move(dragging, f.field_id)} onDragEnd={() => setDragging(null)}>
                <button type="button" className={styles.iconButton} aria-label={f.is_visible ? `Hide ${meta.label}` : `Show ${meta.label}`} aria-pressed={f.is_visible} disabled={isLabel} onClick={() => setFields(working.fields.map((x) => (x.field_id === f.field_id ? { ...x, is_visible: !x.is_visible } : x)))}>
                  {f.is_visible ? "◉" : "○"}
                </button>
                <span className={`${styles.name} ${f.is_visible ? "" : styles.hidden}`}>{meta.label}</span>
                <button type="button" className={styles.iconButton} aria-label="Move up" disabled={isLabel || i <= 1} onClick={() => move(f.field_id, working.fields[i - 1].field_id)}>
                  ▲
                </button>
                <button type="button" className={styles.iconButton} aria-label="Move down" disabled={isLabel || i >= working.fields.length - 1} onClick={() => move(working.fields[i + 1].field_id, f.field_id)}>
                  ▼
                </button>
              </div>
            );
          })}
        </div>
      </div>
    );
  }
  if (section === "group") {
    return (
      <div className={`glass ${styles.panel} ${styles.panelRight}`}>
        <div className={styles.row}>
          <button type="button" className={styles.iconButton} aria-label="Back" onClick={() => setSection(null)}>
            ‹
          </button>
          <p className={styles.panelTitle}>{kanban ? "Columns" : "Group"}</p>
        </div>
        <select
          className="input"
          aria-label={kanban ? "Columns" : "Group by"}
          value={(kanban ? current.view.kanban_field_id : working.groupBy) ?? ""}
          onChange={(e) => {
            if (kanban) patchNow({ kanban_field_id: e.target.value || null });
            onChange({ ...working, groupBy: kanban ? working.groupBy : e.target.value || null, groups: [] });
          }}
        >
          {!kanban && <option value="">None</option>}
          {selects.map((f) => (
            <option key={f.id} value={f.id}>
              {f.label}
            </option>
          ))}
        </select>
        {groupField && (
          <>
            <p className={styles.panelTitle}>Sort</p>
            <div className="segmented" role="group" aria-label="Sort groups">
              {(
                [
                  ["MANUAL", "Manual"],
                  ["ASC", "A → Z"],
                  ["DESC", "Z → A"],
                ] as const
              ).map(([v, l]) => (
                <button key={v} type="button" aria-pressed={working.groupSort === v} onClick={() => onChange({ ...working, groupSort: v })}>
                  {l}
                </button>
              ))}
            </div>
            <p className={styles.panelTitle}>Visible groups</p>
            <div className={styles.list}>
              {sortedGroups.map((g, i) => (
                <div key={g.field_value} className={styles.listItem}>
                  <button type="button" className={styles.iconButton} aria-label={g.is_visible ? `Hide ${labelOfGroup(g.field_value)}` : `Show ${labelOfGroup(g.field_value)}`} aria-pressed={g.is_visible} onClick={() => setGroups(sortedGroups.map((x) => (x.field_value === g.field_value ? { ...x, is_visible: !x.is_visible } : x)))}>
                    {g.is_visible ? "◉" : "○"}
                  </button>
                  <span className={`${styles.name} ${g.is_visible ? "" : styles.hidden}`}>{labelOfGroup(g.field_value)}</span>
                  {working.groupSort === "MANUAL" && (
                    <>
                      <button type="button" className={styles.iconButton} aria-label="Move up" disabled={i === 0} onClick={() => setGroups(swap(sortedGroups, i, i - 1))}>
                        ▲
                      </button>
                      <button type="button" className={styles.iconButton} aria-label="Move down" disabled={i === sortedGroups.length - 1} onClick={() => setGroups(swap(sortedGroups, i, i + 1))}>
                        ▼
                      </button>
                    </>
                  )}
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    );
  }
  return (
    <div className={`glass ${styles.panel} ${styles.panelRight}`}>
      <button type="button" className={styles.listButton} onClick={() => setSection("fields")}>
        Fields <span className={styles.muted}>· {working.fields.filter((f) => f.is_visible).length} shown</span>
      </button>
      {!calendar && (
        <button type="button" className={styles.listButton} onClick={() => setSection("group")}>
          {kanban ? "Columns" : "Group"} <span className={styles.muted}>· {groupField ? `by ${groupField.label}` : "none"}</span>
        </button>
      )}
      <button type="button" className={styles.listButton} onClick={() => transfer("import")}>
        Import records
      </button>
      <button type="button" className={styles.listButton} onClick={() => transfer("export")}>
        Export view <span className={styles.muted}>· CSV</span>
      </button>
      {kanban && (
        <div className={styles.row}>
          <span className={styles.name}>Aggregate</span>
          <select className="input" aria-label="Aggregate" value={current.view.kanban_aggregate_operation ?? ""} onChange={(e) => patchNow({ kanban_aggregate_operation: e.target.value || null })}>
            <option value="">None</option>
            <option value="COUNT">Count</option>
            <option value="SUM">Sum</option>
            <option value="AVG">Average</option>
            <option value="MIN">Min</option>
            <option value="MAX">Max</option>
          </select>
          <select className="input" aria-label="Aggregate field" value={current.view.kanban_aggregate_field_id ?? ""} onChange={(e) => patchNow({ kanban_aggregate_field_id: e.target.value || null })}>
            <option value="">Field…</option>
            {numbers.map((f) => (
              <option key={f.id} value={f.id}>
                {f.label}
              </option>
            ))}
          </select>
        </div>
      )}
      {calendar && (
        <div className={styles.row}>
          <span className={styles.name}>Date field</span>
          <select className="input" aria-label="Date field" value={current.view.calendar_field_id ?? ""} onChange={(e) => patchNow({ calendar_field_id: e.target.value || null })}>
            <option value="">Pick…</option>
            {dates.map((f) => (
              <option key={f.id} value={f.id}>
                {f.label}
              </option>
            ))}
          </select>
        </div>
      )}
      <div className={styles.row}>
        <span className={styles.name}>Compact view</span>
        <OnOff label="Compact view" value={current.view.is_compact} disabled={update.isPending} onChange={(v) => patchNow({ is_compact: v })} />
      </div>
      <div className={styles.row}>
        <span className={styles.name}>Open record in</span>
        <div className="segmented" role="group" aria-label="Open record in">
          <button type="button" aria-pressed={current.view.open_record_in === "SIDE_PANEL"} onClick={() => patchNow({ open_record_in: "SIDE_PANEL" })}>
            Side panel
          </button>
          <button type="button" aria-pressed={current.view.open_record_in === "RECORD_PAGE"} onClick={() => patchNow({ open_record_in: "RECORD_PAGE" })}>
            Record page
          </button>
        </div>
      </div>
      <div className={styles.row}>
        <span className={styles.name}>Visibility</span>
        <div className="segmented" role="group" aria-label="Visibility">
          <button type="button" aria-pressed={current.view.visibility === "WORKSPACE"} disabled={current.view.key === "INDEX"} onClick={() => patchNow({ visibility: "WORKSPACE" })}>
            Workspace
          </button>
          <button type="button" aria-pressed={current.view.visibility === "UNLISTED"} disabled={current.view.key === "INDEX"} onClick={() => patchNow({ visibility: "UNLISTED" })}>
            Unlisted
          </button>
        </div>
      </div>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
    </div>
  );
}

function swap<T>(items: T[], a: number, b: number): T[] {
  const next = [...items];
  [next[a], next[b]] = [next[b], next[a]];
  return next;
}
