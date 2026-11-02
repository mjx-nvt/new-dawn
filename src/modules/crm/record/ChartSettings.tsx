import { useObject, useObjects, type FieldMeta, type ObjectDef } from "../../../kernel/bridge/records";
import { useViews } from "../../../kernel/bridge/views";
import OnOff from "../../workspace-settings/OnOff";
import { RuleEditor } from "../views/FilterEditor";
import { newRule, nextPosition, usableFields, type Rule } from "../views/useViewState";
import styles from "./Record.module.css";

/** Twenty's chart settings (D-067): what a Chart, Table or iFrame widget keeps in its config. */
export interface ChartWidgetConfig {
  /** bar · line · pie · aggregate */
  kind?: string;
  /** On a dashboard: the object's name. */
  source?: string;
  /** On a record page: the one-to-many relation whose records are charted. */
  relation?: string;
  rules?: Rule[];
  x?: { field: string; granularity?: string | null } | null;
  y?: { op: string; field?: string | null; sub?: string | null; ratio_value?: string | null };
  group?: { field: string; granularity?: string | null } | null;
  /** POSITION · ALPHABETICAL · VALUE_DESC · VALUE_ASC */
  sort?: string;
  omit_zero?: boolean;
  cumulative?: boolean;
  horizontal?: boolean;
  stacked?: boolean;
  labels?: boolean;
  legend?: boolean;
  /** none · x · y · both */
  axisName?: string;
  prefix?: string;
  suffix?: string;
  showCenter?: boolean;
  hideEmpty?: boolean;
}

export const OPERATIONS: [string, string][] = [
  ["COUNT", "Count all"],
  ["COUNT_EMPTY", "Count empty"],
  ["COUNT_NOT_EMPTY", "Count not empty"],
  ["COUNT_UNIQUE", "Count unique values"],
  ["SUM", "Sum"],
  ["AVG", "Average"],
  ["MIN", "Min"],
  ["MAX", "Max"],
  ["PERCENT_EMPTY", "Percent empty"],
  ["PERCENT_NOT_EMPTY", "Percent not empty"],
  ["COUNT_TRUE", "Count true"],
  ["COUNT_FALSE", "Count false"],
  ["RATIO", "Ratio"],
];

export const GRANULARITIES: [string, string][] = [
  ["DAY", "Day"],
  ["WEEK", "Week"],
  ["MONTH", "Month"],
  ["QUARTER", "Quarter"],
  ["YEAR", "Year"],
  ["DAY_OF_WEEK", "Day of week"],
  ["MONTH_OF_YEAR", "Month of year"],
  ["QUARTER_OF_YEAR", "Quarter of year"],
];

export function opLabel(op: string): string {
  return OPERATIONS.find(([v]) => v === op)?.[1] ?? op;
}

export function chartConfigOf(config: Record<string, unknown>): ChartWidgetConfig {
  return config as ChartWidgetConfig;
}

const NUMERIC = ["NUMBER", "CURRENCY", "RATING"];
const DATES = ["DATE", "DATE_TIME"];

/** The operations a field's type allows (Twenty's: counts everywhere; maths on numbers;
 *  true / false on booleans; a ratio on selects and booleans). */
function opsFor(field: FieldMeta | undefined): [string, string][] {
  const counts = OPERATIONS.filter(([v]) => v.startsWith("COUNT") && v !== "COUNT_TRUE" && v !== "COUNT_FALSE");
  const percents = OPERATIONS.filter(([v]) => v.startsWith("PERCENT"));
  if (!field) return [["COUNT", "Count all"]];
  const pick = (ops: string[]) => OPERATIONS.filter(([v]) => ops.includes(v));
  if (NUMERIC.includes(field.type)) return [...counts, ...pick(["SUM", "AVG", "MIN", "MAX"]), ...percents];
  if (DATES.includes(field.type)) return [...counts, ...percents];
  if (field.type === "BOOLEAN") return [...counts, ...pick(["COUNT_TRUE", "COUNT_FALSE", "RATIO"])];
  if (field.type === "SELECT" || field.type === "MULTI_SELECT") return [...counts, ...percents, ...pick(["RATIO"])];
  return [...counts, ...percents];
}

/** The fields a chart can group by: selects, dates, booleans, many-to-one relations, text. */
function dimensionFields(def: ObjectDef): FieldMeta[] {
  return def.fields.filter((f) => f.is_active && !f.is_system && (["SELECT", "BOOLEAN", "TEXT", "NUMBER", ...DATES].includes(f.type) || (f.type === "RELATION" && f.options.relation?.kind === "MANY_TO_ONE")));
}

function measureFields(def: ObjectDef): FieldMeta[] {
  return def.fields.filter((f) => f.is_active && f.type !== "POSITION" && f.type !== "ACTOR" && !(f.type === "RELATION" && f.options.relation?.kind !== "MANY_TO_ONE"));
}

function Row({ label, htmlFor, children }: { label: string; htmlFor?: string; children: React.ReactNode }) {
  return (
    <div className={styles.settingsRow}>
      <label className={styles.fieldLabel} htmlFor={htmlFor}>
        {label}
      </label>
      {children}
    </div>
  );
}

/** The Chart widget's settings — Data, then Style — on a dashboard (any object) or a record
 *  page (the records a relation of this object points at). */
export function ChartSettingsForm({ slug, object, onDashboard, config, set }: { slug: string; object: ObjectDef; onDashboard: boolean; config: ChartWidgetConfig; set: (patch: Partial<ChartWidgetConfig>) => void }) {
  const objects = useObjects(slug);
  const relations = object.fields.filter((f) => f.is_active && !f.is_system && f.type === "RELATION" && f.options.relation?.kind === "ONE_TO_MANY" && !f.options.relation.junction);
  const relation = onDashboard ? undefined : relations.find((f) => f.name === config.relation);
  const sourceName = onDashboard ? config.source : relation?.options.relation?.target;
  const source = useObject(slug, sourceName);
  const def = source.data;
  const kind = config.kind ?? "bar";
  const rules = config.rules ?? [];
  const yField = def?.fields.find((f) => f.name === config.y?.field);
  const xField = def?.fields.find((f) => f.name === config.x?.field);
  const gField = def?.fields.find((f) => f.name === config.group?.field);
  const ops = opsFor(yField);
  const setRules = (next: Rule[]) => set({ rules: next });
  return (
    <>
      <Row label="Chart" htmlFor="chart-kind">
        <select id="chart-kind" className="input" value={kind} onChange={(e) => set({ kind: e.target.value })}>
          <option value="bar">Bar</option>
          <option value="line">Line</option>
          <option value="pie">Pie</option>
          <option value="aggregate">Aggregate</option>
        </select>
      </Row>
      <Row label="Source" htmlFor="chart-source">
        {onDashboard ? (
          <select id="chart-source" className="input" value={config.source ?? ""} onChange={(e) => set({ source: e.target.value || undefined, rules: [], x: null, y: { op: "COUNT" }, group: null })}>
            <option value="">Pick an object…</option>
            {(objects.data ?? [])
              .filter((o) => !o.is_system && o.is_active && o.name_singular !== "dashboard")
              .map((o) => (
                <option key={o.id} value={o.name_singular}>
                  {o.label_plural}
                </option>
              ))}
          </select>
        ) : (
          <select id="chart-source" className="input" value={config.relation ?? ""} onChange={(e) => set({ relation: e.target.value || undefined, rules: [], x: null, y: { op: "COUNT" }, group: null })}>
            <option value="">Pick related records…</option>
            {relations.map((f) => (
              <option key={f.id} value={f.name}>
                {f.label}
              </option>
            ))}
          </select>
        )}
      </Row>
      {def && (
        <>
          <div className={styles.settingsRow}>
            <span className={styles.fieldLabel}>Filter</span>
            <div className={styles.rules}>
              {rules.map((r, i) => (
                <RuleEditor key={`${r.field_id}-${i}`} slug={slug} object={def} rule={r} onChange={(next) => setRules(rules.map((x, j) => (j === i ? next : x)))} onRemove={() => setRules(rules.filter((_, j) => j !== i))} />
              ))}
              <button type="button" className="btn btnGhost btnCompact" disabled={!usableFields(def).length} onClick={() => setRules([...rules, newRule(usableFields(def)[0], null, nextPosition(rules))])}>
                + Add filter
              </button>
            </div>
          </div>
          {kind !== "aggregate" && (
            <>
              <Row label={kind === "pie" ? "Each slice represents" : "X axis"} htmlFor="chart-x">
                <select id="chart-x" className="input" value={config.x?.field ?? ""} onChange={(e) => set({ x: e.target.value ? { field: e.target.value, granularity: config.x?.granularity ?? null } : null })}>
                  <option value="">Pick a field…</option>
                  {dimensionFields(def).map((f) => (
                    <option key={f.id} value={f.name}>
                      {f.label}
                    </option>
                  ))}
                </select>
              </Row>
              {xField && DATES.includes(xField.type) && (
                <Row label="Date granularity" htmlFor="chart-x-granularity">
                  <select id="chart-x-granularity" className="input" value={config.x?.granularity ?? "MONTH"} onChange={(e) => set({ x: { field: xField.name, granularity: e.target.value } })}>
                    {GRANULARITIES.map(([v, l]) => (
                      <option key={v} value={v}>
                        {l}
                      </option>
                    ))}
                  </select>
                </Row>
              )}
            </>
          )}
          <Row label={kind === "aggregate" ? "Data on display" : "Y axis"} htmlFor="chart-op">
            <select id="chart-op" className="input" value={config.y?.op ?? "COUNT"} onChange={(e) => set({ y: { ...(config.y ?? { op: "COUNT" }), op: e.target.value } })}>
              {ops.map(([v, l]) => (
                <option key={v} value={v}>
                  {l}
                </option>
              ))}
            </select>
          </Row>
          <Row label="Field" htmlFor="chart-y-field">
            <select id="chart-y-field" className="input" value={config.y?.field ?? ""} onChange={(e) => set({ y: { ...(config.y ?? { op: "COUNT" }), field: e.target.value || null, ratio_value: null } })}>
              <option value="">— (count of records)</option>
              {measureFields(def).map((f) => (
                <option key={f.id} value={f.name}>
                  {f.label}
                </option>
              ))}
            </select>
          </Row>
          {config.y?.op === "RATIO" && yField && (
            <Row label="Ratio of" htmlFor="chart-ratio">
              <select id="chart-ratio" className="input" value={config.y?.ratio_value ?? ""} onChange={(e) => set({ y: { ...(config.y ?? { op: "RATIO" }), ratio_value: e.target.value || null } })}>
                <option value="">Pick a value…</option>
                {yField.type === "BOOLEAN" ? (
                  <>
                    <option value="true">True</option>
                    <option value="false">False</option>
                  </>
                ) : (
                  (yField.options.select ?? []).map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))
                )}
              </select>
            </Row>
          )}
          {kind !== "aggregate" && kind !== "pie" && (
            <>
              <Row label="Group by" htmlFor="chart-group">
                <select id="chart-group" className="input" value={config.group?.field ?? ""} onChange={(e) => set({ group: e.target.value ? { field: e.target.value, granularity: config.group?.granularity ?? null } : null })}>
                  <option value="">None</option>
                  {dimensionFields(def).map((f) => (
                    <option key={f.id} value={f.name}>
                      {f.label}
                    </option>
                  ))}
                </select>
              </Row>
              {gField && DATES.includes(gField.type) && (
                <Row label="Group granularity" htmlFor="chart-g-granularity">
                  <select id="chart-g-granularity" className="input" value={config.group?.granularity ?? "MONTH"} onChange={(e) => set({ group: { field: gField.name, granularity: e.target.value } })}>
                    {GRANULARITIES.map(([v, l]) => (
                      <option key={v} value={v}>
                        {l}
                      </option>
                    ))}
                  </select>
                </Row>
              )}
              <Row label="Sort by" htmlFor="chart-sort">
                <select id="chart-sort" className="input" value={config.sort ?? "POSITION"} onChange={(e) => set({ sort: e.target.value })}>
                  <option value="POSITION">Position</option>
                  <option value="ALPHABETICAL">Alphabetical</option>
                  <option value="VALUE_DESC">By value, highest first</option>
                  <option value="VALUE_ASC">By value, lowest first</option>
                </select>
              </Row>
              <div className={styles.settingsRow}>
                <span className={styles.fieldLabel}>Omit zero values</span>
                <OnOff label="Omit zero values" value={!!config.omit_zero} onChange={(v) => set({ omit_zero: v })} />
              </div>
              <div className={styles.settingsRow}>
                <span className={styles.fieldLabel}>Cumulative</span>
                <OnOff label="Cumulative" value={!!config.cumulative} onChange={(v) => set({ cumulative: v })} />
              </div>
            </>
          )}
          {kind === "bar" && (
            <div className={styles.settingsRow}>
              <span className={styles.fieldLabel}>Horizontal bars</span>
              <OnOff label="Horizontal bars" value={!!config.horizontal} onChange={(v) => set({ horizontal: v })} />
            </div>
          )}
          {(kind === "bar" || kind === "line") && config.group?.field && (
            <div className={styles.settingsRow}>
              <span className={styles.fieldLabel}>Stacked</span>
              <OnOff label="Stacked" value={!!config.stacked} onChange={(v) => set({ stacked: v })} />
            </div>
          )}
          {(kind === "bar" || kind === "line") && (
            <>
              <div className={styles.settingsRow}>
                <span className={styles.fieldLabel}>Data labels</span>
                <OnOff label="Data labels" value={!!config.labels} onChange={(v) => set({ labels: v })} />
              </div>
              {config.group?.field && (
                <div className={styles.settingsRow}>
                  <span className={styles.fieldLabel}>Legend</span>
                  <OnOff label="Legend" value={config.legend !== false} onChange={(v) => set({ legend: v })} />
                </div>
              )}
              <Row label="Axis name" htmlFor="chart-axis">
                <select id="chart-axis" className="input" value={config.axisName ?? "none"} onChange={(e) => set({ axisName: e.target.value })}>
                  <option value="none">None</option>
                  <option value="x">X axis</option>
                  <option value="y">Y axis</option>
                  <option value="both">Both</option>
                </select>
              </Row>
            </>
          )}
          {kind === "pie" && (
            <>
              <div className={styles.settingsRow}>
                <span className={styles.fieldLabel}>Show value in centre</span>
                <OnOff label="Show value in centre" value={config.showCenter !== false} onChange={(v) => set({ showCenter: v })} />
              </div>
              <div className={styles.settingsRow}>
                <span className={styles.fieldLabel}>Hide empty category</span>
                <OnOff label="Hide empty category" value={!!config.hideEmpty} onChange={(v) => set({ hideEmpty: v })} />
              </div>
            </>
          )}
          <Row label="Prefix" htmlFor="chart-prefix">
            <input id="chart-prefix" className="input" value={config.prefix ?? ""} placeholder="$" onChange={(e) => set({ prefix: e.target.value })} />
          </Row>
          <Row label="Suffix" htmlFor="chart-suffix">
            <input id="chart-suffix" className="input" value={config.suffix ?? ""} placeholder="%" onChange={(e) => set({ suffix: e.target.value })} />
          </Row>
        </>
      )}
    </>
  );
}

/** The Table widget's settings: an object's view, and whether its cells edit. */
export function TableSettingsForm({ slug, config, set }: { slug: string; config: Record<string, unknown>; set: (patch: Record<string, unknown>) => void }) {
  const objects = useObjects(slug);
  const objectName = typeof config.object === "string" ? config.object : "";
  const views = useViews(slug, objectName || undefined);
  return (
    <>
      <Row label="Object" htmlFor="table-object">
        <select id="table-object" className="input" value={objectName} onChange={(e) => set({ object: e.target.value || null, view: null })}>
          <option value="">Pick an object…</option>
          {(objects.data ?? [])
            .filter((o) => !o.is_system && o.is_active && o.name_singular !== "dashboard")
            .map((o) => (
              <option key={o.id} value={o.name_singular}>
                {o.label_plural}
              </option>
            ))}
        </select>
      </Row>
      <Row label="View" htmlFor="table-view">
        <select id="table-view" className="input" value={typeof config.view === "string" ? config.view : ""} disabled={!objectName} onChange={(e) => set({ view: e.target.value || null })}>
          <option value="">Pick a view…</option>
          {(views.data ?? []).map((v) => (
            <option key={v.id} value={v.id}>
              {v.name}
            </option>
          ))}
        </select>
      </Row>
      <div className={styles.settingsRow}>
        <span className={styles.fieldLabel}>Allow editing</span>
        <OnOff label="Allow editing" value={config.editable === true} onChange={(v) => set({ editable: v })} />
      </div>
    </>
  );
}

/** The iFrame widget's settings: a localhost address. */
export function IframeSettingsForm({ config, set }: { config: Record<string, unknown>; set: (patch: Record<string, unknown>) => void }) {
  return (
    <Row label="Address" htmlFor="iframe-url">
      <input id="iframe-url" className="input" value={typeof config.url === "string" ? config.url : ""} placeholder="http://localhost:3000/…" onChange={(e) => set({ url: e.target.value })} />
      <span className="fieldHint">Only a localhost address — nothing on the web is shown (D-067).</span>
    </Row>
  );
}
