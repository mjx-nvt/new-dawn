import { useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { useResetRecordLayout, useSaveRecordLayout, type Layout, type Widget } from "../../../kernel/bridge/layouts";
import { hasColumns, type ObjectDef } from "../../../kernel/bridge/records";
import DangerButton from "../../../kernel/shell/DangerButton";
import type { LayoutInput } from "../../../bindings";
import OnOff from "../../workspace-settings/OnOff";
import { chartConfigOf, ChartSettingsForm, IframeSettingsForm, TableSettingsForm } from "./ChartSettings";
import styles from "./Record.module.css";
import { configOf, typeLabel, WIDGET_TYPES, WidgetBody, WidgetCard, type WidgetContext } from "./widgets";

interface EditTab {
  key: string;
  title: string;
  is_default: boolean;
  widgets: Widget[];
}

let seq = 0;
const fresh = () => `new-${++seq}`;

/** Twenty's layout editor (D-062), on the page itself: widgets dragged by their handle and
 *  resized by their corner, snapped to the 12-column grid; a widget's settings (title, fields,
 *  relation, text); tabs added, renamed, reordered, removed, one made default; Save writes the
 *  whole layout at once, Cancel drops the changes, Reset brings Twenty's default back. */
/** A dashboard's widgets (D-067): the ones that need no record. */
const DASHBOARD_TYPES = ["CHART", "TABLE", "RICH_TEXT", "IFRAME"];

export default function LayoutEditor({ ctx, layout, onDone, kind = "record", onSave }: { ctx: WidgetContext; layout: Layout; onDone: () => void; kind?: "record" | "dashboard"; onSave?: (input: LayoutInput) => Promise<unknown> }) {
  const [trouble, setTrouble] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const { slug, object } = ctx;
  const [tabs, setTabs] = useState<EditTab[]>(() => layout.tabs.map((t) => ({ key: t.tab.id, title: t.tab.title, is_default: t.tab.is_default, widgets: t.widgets.map((w) => ({ ...w, config: configOf(w) })) })));
  const [at, setAt] = useState(() => Math.max(0, layout.tabs.findIndex((t) => t.tab.is_default)));
  const [picked, setPicked] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  const save = useSaveRecordLayout(slug, object.meta.name_singular);
  const reset = useResetRecordLayout(slug, object.meta.name_singular);
  const tab = tabs[at] ?? tabs[0];
  const patchTab = (i: number, fn: (t: EditTab) => EditTab) => setTabs((ts) => ts.map((t, j) => (j === i ? fn(t) : t)));
  const patchWidget = (id: string, fn: (w: Widget) => Widget) => patchTab(at, (t) => ({ ...t, widgets: t.widgets.map((w) => (w.id === id ? fn(w) : w)) }));
  const moveTab = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= tabs.length) return;
    setTabs((ts) => {
      const next = [...ts];
      [next[i], next[j]] = [next[j], next[i]];
      return next;
    });
    setAt(j);
  };
  const addTab = () => {
    setTabs((ts) => [...ts, { key: fresh(), title: `Tab ${ts.length + 1}`, is_default: false, widgets: [] }]);
    setAt(tabs.length);
  };
  const removeTab = (i: number) => {
    if (tabs.length <= 1) return;
    setTabs((ts) => ts.filter((_, j) => j !== i));
    setAt(Math.max(0, i - 1));
    setPicked(null);
  };
  const addWidget = (type: string) => {
    const y = tab.widgets.reduce((m, w) => Math.max(m, w.grid_y + w.grid_h), 0);
    const made: Widget = { id: fresh(), tab_id: tab.key, type, title: typeLabel(type), position: tab.widgets.length, grid_x: 0, grid_y: y, grid_w: type === "CHART" ? 6 : 12, grid_h: 6, config: type === "FIELDS" ? { fields: [] } : type === "RELATED" ? { editable: true } : type === "CHART" ? { kind: "bar", y: { op: "COUNT" } } : type === "TABLE" ? { editable: false } : {} };
    patchTab(at, (t) => ({ ...t, widgets: [...t.widgets, made] }));
    setPicked(made.id);
    setAdding(false);
  };
  const removeWidget = (id: string) => {
    patchTab(at, (t) => ({ ...t, widgets: t.widgets.filter((w) => w.id !== id) }));
    setPicked(null);
  };

  /** Drags a widget (move) or its corner (resize), snapped to the grid's cells. */
  const drag = (e: ReactPointerEvent<HTMLElement>, w: Widget, mode: "move" | "resize") => {
    e.preventDefault();
    e.stopPropagation();
    const el = grid.current;
    if (!el) return;
    // The grid's own metrics (the stylesheet's rows and gap), so snapping never drifts from it.
    const style = getComputedStyle(el);
    const gap = parseFloat(style.columnGap) || 16;
    const row = (parseFloat(style.gridAutoRows) || 56) + (parseFloat(style.rowGap) || gap);
    const cell = (el.clientWidth + gap) / 12;
    const start = { x: e.clientX, y: e.clientY, gx: w.grid_x, gy: w.grid_y, gw: w.grid_w, gh: w.grid_h };
    const move = (ev: PointerEvent) => {
      const dx = Math.round((ev.clientX - start.x) / cell);
      const dy = Math.round((ev.clientY - start.y) / row);
      patchWidget(w.id, (cur) =>
        mode === "move"
          ? { ...cur, grid_x: Math.max(0, Math.min(12 - cur.grid_w, start.gx + dx)), grid_y: Math.max(0, start.gy + dy) }
          : { ...cur, grid_w: Math.max(1, Math.min(12 - cur.grid_x, start.gw + dx)), grid_h: Math.max(1, Math.min(60, start.gh + dy)) },
      );
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      patchTab(at, (t) => ({ ...t, widgets: settle(t.widgets) }));
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };

  const submit = () => {
    const input: LayoutInput = {
      tabs: tabs.map((t, i) => ({
        title: t.title,
        position: i,
        is_default: t.is_default,
        widgets: t.widgets.map((w, j) => ({ type: w.type, title: w.title, position: j, grid_x: w.grid_x, grid_y: w.grid_y, grid_w: w.grid_w, grid_h: w.grid_h, config: w.config })),
      })),
    };
    setTrouble(null);
    if (onSave) {
      setSaving(true);
      onSave(input)
        .then(onDone)
        .catch((e: Error) => setTrouble(e.message))
        .finally(() => setSaving(false));
    } else save.mutate(input, { onSuccess: onDone });
  };
  const person = object.meta.name_singular === "person";
  const available = kind === "dashboard" ? WIDGET_TYPES.filter((t) => DASHBOARD_TYPES.includes(t.type)) : WIDGET_TYPES.filter((t) => !t.since && !["TABLE", "IFRAME"].includes(t.type) && (person || !["COMMITMENTS", "MEETINGS"].includes(t.type)));
  const later = kind === "dashboard" ? [] : WIDGET_TYPES.filter((t) => t.since);
  const pickedWidget = tab.widgets.find((w) => w.id === picked);
  const mutationError = save.error ?? reset.error ?? (trouble ? new Error(trouble) : null);
  const busy = save.isPending || saving;

  return (
    <div>
      <div className={styles.editBar} role="toolbar" aria-label="Layout editor">
        <span className={styles.editTitle}>{kind === "dashboard" ? "Editing the dashboard" : `Editing the ${object.meta.label_singular.toLowerCase()} page`}</span>
        <button type="button" className="btn btnSecondary btnCompact" aria-expanded={adding} onClick={() => setAdding(!adding)}>
          + Widget
        </button>
        <span className={styles.spacer} />
        {kind === "record" && (
          <DangerButton confirm="Back to the default page?" disabled={reset.isPending} onConfirm={() => reset.mutate(undefined, { onSuccess: onDone })}>
            Reset to default
          </DangerButton>
        )}
        <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>
          Cancel
        </button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={busy} onClick={submit}>
          {busy ? "Saving…" : "Save"}
        </button>
      </div>
      {mutationError && (
        <p className="formError" role="alert">
          {mutationError.message}
        </p>
      )}
      {adding && (
        <div className={styles.picker} role="group" aria-label="Widget types">
          {available.map((t) => (
            <button key={t.type} type="button" className="btn btnSecondary btnCompact" onClick={() => addWidget(t.type)}>
              {t.label}
            </button>
          ))}
          {later.map((t) => (
            <button key={t.type} type="button" className="btn btnGhost btnCompact" disabled title={`Arrives with ${t.since}`}>
              {t.label}
            </button>
          ))}
        </div>
      )}
      <div className={`${styles.tabs} ${styles.tabsEdit}`} role="tablist" aria-label="Tabs">
        {tabs.map((t, i) => (
          <div key={t.key} className={i === at ? `${styles.tabEdit} ${styles.tabEditActive}` : styles.tabEdit}>
            {i === at ? (
              <input className={`input ${styles.tabInput}`} aria-label="Tab title" value={t.title} onChange={(e) => patchTab(i, (x) => ({ ...x, title: e.target.value }))} />
            ) : (
              <button type="button" role="tab" aria-selected={false} className="btn btnGhost btnCompact" onClick={() => { setAt(i); setPicked(null); }}>
                {t.title}
              </button>
            )}
            {i === at && (
              <>
                <button type="button" className="btn btnGhost btnCompact" aria-label="Move tab left" disabled={i === 0} onClick={() => moveTab(i, -1)}>
                  ‹
                </button>
                <button type="button" className="btn btnGhost btnCompact" aria-label="Move tab right" disabled={i === tabs.length - 1} onClick={() => moveTab(i, 1)}>
                  ›
                </button>
                <button type="button" className={t.is_default ? "btn btnSecondary btnCompact" : "btn btnGhost btnCompact"} aria-pressed={t.is_default} onClick={() => setTabs((ts) => ts.map((x, j) => ({ ...x, is_default: j === i })))}>
                  Default
                </button>
                <button type="button" className="btn btnDanger btnCompact" aria-label="Remove tab" disabled={tabs.length <= 1} onClick={() => removeTab(i)}>
                  ×
                </button>
              </>
            )}
          </div>
        ))}
        <button type="button" className="btn btnGhost btnCompact" onClick={addTab}>
          + Tab
        </button>
      </div>
      {pickedWidget && <WidgetSettings slug={slug} object={object} onDashboard={kind === "dashboard"} widget={pickedWidget} onChange={(fn) => patchWidget(pickedWidget.id, fn)} onRemove={() => removeWidget(pickedWidget.id)} onClose={() => setPicked(null)} />}
      <div ref={grid} className={styles.grid}>
        {tab.widgets.map((w) => (
          <WidgetCard
            key={w.id}
            widget={w}
            className={w.id === picked ? styles.widgetPicked : styles.widgetEditable}
            tools={
              <span className={styles.widgetTools}>
                <button type="button" className={styles.handle} aria-label={`Move ${w.title || typeLabel(w.type)}`} onPointerDown={(e) => drag(e, w, "move")}>
                  ⠿
                </button>
                <button type="button" className="btn btnGhost btnCompact" aria-pressed={w.id === picked} onClick={() => setPicked(w.id === picked ? null : w.id)}>
                  Settings
                </button>
              </span>
            }
            after={<span className={styles.resize} role="presentation" onPointerDown={(e) => drag(e, w, "resize")} />}
          >
            <WidgetBody ctx={ctx} widget={w} />
          </WidgetCard>
        ))}
        {tab.widgets.length === 0 && (
          <div className={`${styles.widgetGhost} ghostCard`}>
            <span className="ghostText">No widgets on this tab — + Widget adds one</span>
          </div>
        )}
      </div>
    </div>
  );
}

/** Widgets pushed down until none overlaps, in reading order. */
// ponytail: naive push-down (the moved widget may be the one pushed); a real compaction if it matters.
function settle(widgets: Widget[]): Widget[] {
  const placed: Widget[] = [];
  for (const w of [...widgets].sort((a, b) => a.grid_y - b.grid_y || a.grid_x - b.grid_x)) {
    let cur = w;
    const overlaps = (o: Widget) => cur.grid_x < o.grid_x + o.grid_w && o.grid_x < cur.grid_x + cur.grid_w && cur.grid_y < o.grid_y + o.grid_h && o.grid_y < cur.grid_y + cur.grid_h;
    while (placed.some(overlaps)) cur = { ...cur, grid_y: cur.grid_y + 1 };
    placed.push(cur);
  }
  return widgets.map((w) => placed.find((p) => p.id === w.id) ?? w);
}

const names = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : []);

/** A widget's settings: its title, and by type the fields shown, the relation and whether its
 *  table edits, or the text. */
function WidgetSettings({ slug, object, onDashboard, widget, onChange, onRemove, onClose }: { slug: string; object: ObjectDef; onDashboard: boolean; widget: Widget; onChange: (fn: (w: Widget) => Widget) => void; onRemove: () => void; onClose: () => void }) {
  const config = configOf(widget);
  const set = (patch: Record<string, unknown>) => onChange((w) => ({ ...w, config: { ...configOf(w), ...patch } }));
  const all = object.fields.filter((f) => f.is_active && !f.is_system && f.type !== "POSITION" && hasColumns(f));
  const relations = object.fields.filter((f) => f.is_active && !f.is_system && f.type === "RELATION" && f.options.relation?.kind === "ONE_TO_MANY");
  const chosen = names(config.fields);
  const toggle = (name: string, on: boolean) => {
    const base = chosen.length ? chosen : all.map((f) => f.name);
    set({ fields: on ? [...base.filter((n) => n !== name), name] : base.filter((n) => n !== name) });
  };
  return (
    <div className={`card ${styles.settings}`} role="group" aria-label={`${widget.title || typeLabel(widget.type)} settings`}>
      <div className={styles.settingsRow}>
        <label className={styles.fieldLabel} htmlFor="widget-title">
          Title
        </label>
        <input id="widget-title" className="input" value={widget.title} onChange={(e) => onChange((w) => ({ ...w, title: e.target.value }))} />
      </div>
      {widget.type === "FIELDS" && (
        <div className={styles.settingsRow}>
          <span className={styles.fieldLabel}>Fields</span>
          <div className={styles.checks}>
            {all.map((f) => (
              <label key={f.id} className={styles.check}>
                <input type="checkbox" checked={chosen.length ? chosen.includes(f.name) : true} onChange={(e) => toggle(f.name, e.target.checked)} />
                {f.label}
              </label>
            ))}
          </div>
        </div>
      )}
      {widget.type === "RELATED" && (
        <>
          <div className={styles.settingsRow}>
            <label className={styles.fieldLabel} htmlFor="widget-relation">
              Relation
            </label>
            <select id="widget-relation" className="input" value={typeof config.field === "string" ? config.field : ""} onChange={(e) => set({ field: e.target.value || null })}>
              <option value="">—</option>
              {relations.map((f) => (
                <option key={f.id} value={f.name}>
                  {f.label}
                </option>
              ))}
            </select>
          </div>
          <div className={styles.settingsRow}>
            <span className={styles.fieldLabel}>Allow editing</span>
            <OnOff label="Allow editing" value={config.editable !== false} onChange={(v) => set({ editable: v })} />
          </div>
        </>
      )}
      {widget.type === "CHART" && <ChartSettingsForm slug={slug} object={object} onDashboard={onDashboard} config={chartConfigOf(config)} set={(patch) => set(patch as Record<string, unknown>)} />}
      {widget.type === "TABLE" && <TableSettingsForm slug={slug} config={config} set={set} />}
      {widget.type === "IFRAME" && <IframeSettingsForm config={config} set={set} />}
      {widget.type === "RICH_TEXT" && (
        <div className={styles.settingsRow}>
          <label className={styles.fieldLabel} htmlFor="widget-text">
            Text
          </label>
          <textarea id="widget-text" className="input" rows={4} value={typeof config.text === "string" ? config.text : ""} onChange={(e) => set({ text: e.target.value })} />
        </div>
      )}
      <div className={styles.settingsActions}>
        <button type="button" className="btn btnDanger btnCompact" onClick={onRemove}>
          Remove widget
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={onClose}>
          Done
        </button>
      </div>
    </div>
  );
}
