import { useNavigate } from "@tanstack/react-router";
import { useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import {
  estimateCalls,
  labelCalls,
  stateCalls,
  useArchivePmProject,
  usePmEstimateMutation,
  usePmEstimates,
  usePmLabelMutation,
  usePmLabels,
  usePmStateMutation,
  usePmStates,
  useUpdatePmProject,
  type EstimateKind,
  type PmLabel,
  type PmProject,
  type PmProjectPatch,
  type PmState,
  type StateGroup,
} from "../../kernel/bridge/pm";
import { COLORS } from "../../kernel/bridge/datamodel";
import { usePeople } from "../../kernel/bridge/people";
import { useDeleteProject } from "../../kernel/bridge/projects";
import { colorOf } from "../../kernel/charts/Charts";
import { timeZones } from "../../kernel/format/format";
import DangerButton from "../../kernel/shell/DangerButton";
import OnOff from "../workspace-settings/OnOff";
import { COVERS, EMOJI, ESTIMATE_PRESETS, FEATURES, GROUPS, whoName } from "./pm";
import styles from "./Projects.module.css";

const SECTIONS: [string, string][] = [
  ["general", "General"],
  ["members", "Members"],
  ["features", "Features"],
  ["states", "States"],
  ["labels", "Labels"],
  ["estimates", "Estimates"],
  ["automations", "Automations"],
];
const MONTHS = [0, 1, 2, 3, 6, 9, 12];

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        {hint && <p className="rowHint">{hint}</p>}
      </div>
      <div className="rowControl">{children}</div>
    </div>
  );
}

/** An IME is still writing (Vietnamese): Enter confirms the composition, not the field. */
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

/** A colour of the palette (Twenty's names, drawn in light and dark), its dot beside it. */
function ColorPick({ label, value, disabled, onChange }: { label: string; value: string; disabled?: boolean; onChange: (c: string) => void }) {
  const known = (COLORS as readonly string[]).includes(value) ? value : "gray";
  return (
    <span className={styles.colorPick}>
      <span className={styles.dot} style={{ background: colorOf(0, known) }} aria-hidden />
      <select className="input" aria-label={label} value={known} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
        {COLORS.map((c) => (
          <option key={c} value={c}>
            {c[0].toUpperCase() + c.slice(1)}
          </option>
        ))}
      </select>
    </span>
  );
}

/** A text kept when you leave it (or press Enter), once, if it changed. Keyed by its value
 *  where it is used, so a change from elsewhere shows. */
function Text({ label, value, multiline, maxLength, placeholder, onSave }: { label: string; value: string; multiline?: boolean; maxLength?: number; placeholder?: string; onSave: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  const saved = useRef(value);
  const save = () => {
    if (draft !== saved.current) {
      saved.current = draft;
      onSave(draft);
    }
  };
  return multiline ? (
    <textarea className="input" aria-label={label} rows={3} value={draft} placeholder={placeholder} onChange={(e) => setDraft(e.target.value)} onBlur={save} />
  ) : (
    <input className="input" aria-label={label} value={draft} maxLength={maxLength} placeholder={placeholder} onChange={(e) => setDraft(e.target.value)} onBlur={save} onKeyDown={(e) => e.key === "Enter" && !composing(e) && save()} />
  );
}

/** A project's settings (D-073, Plane's): General, Members, Features, States, Labels, Estimates
 *  and Automations — each change saved as it is made. */
export default function ProjectSettings({ slug, project: p, section }: { slug: string; project: PmProject; section?: string }) {
  const navigate = useNavigate();
  const update = useUpdatePmProject(slug);
  const current = SECTIONS.find(([s]) => s === section)?.[0] ?? "general";
  const save = (patch: PmProjectPatch) => update.mutate({ id: p.id, patch });
  return (
    <section className="section">
      <div className="segmented" role="tablist" aria-label="Settings">
        {SECTIONS.map(([s, label]) => (
          <button key={s} type="button" role="tab" aria-selected={s === current} onClick={() => void navigate({ to: "/projects/$projectId", params: { projectId: p.id }, search: { tab: "settings", section: s }, replace: true })}>
            {label}
          </button>
        ))}
      </div>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
      <div className={styles.settingsBody}>
        {current === "general" && <General slug={slug} project={p} save={save} />}
        {current === "members" && <Members slug={slug} project={p} busy={update.isPending} save={save} />}
        {current === "features" && (
          <div className="group">
            {FEATURES.map((f) => (
              <Row key={f.key} label={f.label} hint={f.hint}>
                <OnOff label={f.label} value={p.features[f.key]} disabled={update.isPending} onChange={(on) => save({ features: { ...p.features, [f.key]: on } })} />
              </Row>
            ))}
          </div>
        )}
        {current === "states" && <States slug={slug} projectId={p.id} />}
        {current === "labels" && <Labels slug={slug} projectId={p.id} />}
        {current === "estimates" && <Estimates slug={slug} projectId={p.id} />}
        {current === "automations" && <Automations slug={slug} project={p} busy={update.isPending} save={save} />}
      </div>
    </section>
  );
}

function General({ slug, project: p, save }: { slug: string; project: PmProject; save: (patch: PmProjectPatch) => void }) {
  const archive = useArchivePmProject(slug);
  const remove = useDeleteProject(slug);
  const navigate = useNavigate();
  const zones = timeZones();
  if (p.time_zone && !zones.includes(p.time_zone)) zones.unshift(p.time_zone);
  return (
    <>
      <div className="group">
        <Row label="Cover">
          <div className="segmented" role="group" aria-label="Cover">
            {COVERS.map((c) => (
              <button key={c.key || "none"} type="button" aria-pressed={p.cover === c.key} onClick={() => save({ cover: c.key })}>
                {c.label}
              </button>
            ))}
          </div>
        </Row>
        <Row label="Logo">
          <div className={styles.emojiRow}>
            <Text key={p.emoji} label="Logo emoji" value={p.emoji} maxLength={8} onSave={(emoji) => save({ emoji })} />
            {EMOJI.slice(0, 5).map((e) => (
              <button key={e} type="button" className="btn btnGhost btnCompact" aria-label={`Logo ${e}`} aria-pressed={p.emoji === e} onClick={() => save({ emoji: e })}>
                {e}
              </button>
            ))}
          </div>
        </Row>
        <Row label="Name">
          <Text key={p.name} label="Name" value={p.name} onSave={(name) => save({ name })} />
        </Row>
        <Row label="Identifier" hint="Its work items are named by it (VIFC-12): up to ten letters or digits, unique here.">
          <Text key={p.identifier} label="Identifier" value={p.identifier} maxLength={10} onSave={(identifier) => save({ identifier })} />
        </Row>
        <Row label="Description">
          <Text key={p.description} label="Description" value={p.description} multiline onSave={(description) => save({ description })} />
        </Row>
        <Row label="Network" hint="A label: nothing leaves this Mac.">
          <div className="segmented" role="group" aria-label="Network">
            <button type="button" aria-pressed={p.network === "private"} onClick={() => save({ network: "private" })}>
              Private
            </button>
            <button type="button" aria-pressed={p.network === "public"} onClick={() => save({ network: "public" })}>
              Public
            </button>
          </div>
        </Row>
        <Row label="Time zone">
          <select className="input" aria-label="Time zone" value={p.time_zone} onChange={(e) => save({ time_zone: e.target.value })}>
            <option value="">Yours (Settings → Formats)</option>
            {zones.map((z) => (
              <option key={z} value={z}>
                {z.replace(/_/g, " ")}
              </option>
            ))}
          </select>
        </Row>
      </div>
      <p className="groupTitle">Archive or delete</p>
      <div className="group">
        <Row label={p.archived_at ? "Restore the project" : "Archive the project"} hint={p.archived_at ? "It comes back to the projects list." : "It leaves the projects list and keeps everything; Archived shows it."}>
          <button type="button" className="btn btnSecondary btnCompact" disabled={archive.isPending} onClick={() => archive.mutate({ id: p.id, archived: !p.archived_at })}>
            {p.archived_at ? "Restore" : "Archive"}
          </button>
        </Row>
        <Row label="Delete the project" hint="Its overview, states, labels, estimates and work items go with it. Its meetings stay.">
          <DangerButton confirm={`Delete “${p.name}”?`} disabled={remove.isPending} onConfirm={() => remove.mutate(p.id, { onSuccess: () => void navigate({ to: "/projects", search: {} }) })}>
            Delete
          </DangerButton>
        </Row>
      </div>
      {(archive.error ?? remove.error) && (
        <p className="formError" role="alert">
          {(archive.error ?? remove.error)?.message}
        </p>
      )}
    </>
  );
}

function Who({ label, value, slug, disabled, onChange }: { label: string; value: string | null; slug: string; disabled?: boolean; onChange: (v: string) => void }) {
  const people = usePeople(slug).data ?? [];
  const gone = !!value && value !== "me" && !people.some((x) => x.id === value);
  return (
    <select className="input" aria-label={label} value={value ?? ""} disabled={disabled} onChange={(e) => onChange(e.target.value)}>
      <option value="">No one</option>
      <option value="me">You</option>
      {gone && value && <option value={value}>{whoName(value, people)}</option>}
      {people.map((x) => (
        <option key={x.id} value={x.id}>
          {x.name}
        </option>
      ))}
    </select>
  );
}

function Members({ slug, project: p, busy, save }: { slug: string; project: PmProject; busy: boolean; save: (patch: PmProjectPatch) => void }) {
  const people = usePeople(slug).data ?? [];
  return (
    <div className="group">
      <Row label="Lead" hint="Who leads the project — you or someone in the people memory.">
        <Who label="Lead" value={p.lead} slug={slug} disabled={busy} onChange={(lead) => save({ lead })} />
      </Row>
      <Row label="Default assignee" hint="New work items go to them.">
        <Who label="Default assignee" value={p.default_assignee} slug={slug} disabled={busy} onChange={(default_assignee) => save({ default_assignee })} />
      </Row>
      <Row label="Default subscribers" hint="They follow every new work item.">
        <div className={styles.emojiRow}>
          {p.subscribers.map((s) => (
            <span key={s} className="chip">
              {whoName(s, people)}{" "}
              <button type="button" className={styles.chipX} aria-label={`Remove ${whoName(s, people)}`} onClick={() => save({ subscribers: p.subscribers.filter((x) => x !== s) })}>
                ✕
              </button>
            </span>
          ))}
          <select className="input" aria-label="Add a subscriber" value="" disabled={busy} onChange={(e) => e.target.value && save({ subscribers: [...p.subscribers, e.target.value] })}>
            <option value="">Add…</option>
            {!p.subscribers.includes("me") && <option value="me">You</option>}
            {people
              .filter((x) => !p.subscribers.includes(x.id))
              .map((x) => (
                <option key={x.id} value={x.id}>
                  {x.name}
                </option>
              ))}
          </select>
        </div>
      </Row>
    </div>
  );
}

function StateRow({ slug, projectId, state: s, many }: { slug: string; projectId: string; state: PmState; many: boolean }) {
  const why = s.is_default ? "The default state stays — make another one the default first" : !many ? "Each group keeps one state" : null;
  const update = usePmStateMutation(slug, projectId, stateCalls.update);
  const setDefault = usePmStateMutation(slug, projectId, stateCalls.setDefault);
  const remove = usePmStateMutation(slug, projectId, stateCalls.remove);
  const move = usePmStateMutation(slug, projectId, stateCalls.move);
  const error = update.error ?? setDefault.error ?? remove.error ?? move.error;
  return (
    <div className="groupRow">
      <div className={styles.stateMain}>
        <ColorPick label={`${s.name} colour`} value={s.color} disabled={update.isPending} onChange={(color) => update.mutate({ id: s.id, patch: { color } })} />
        <div className={styles.stateText}>
          <Text key={s.name} label={`${s.name} name`} value={s.name} placeholder="Name" onSave={(name) => update.mutate({ id: s.id, patch: { name } })} />
          <Text key={`d-${s.description}`} label={`${s.name} description`} value={s.description} placeholder="Add a description" onSave={(description) => update.mutate({ id: s.id, patch: { description } })} />
          {error && <p className="formError">{error.message}</p>}
        </div>
      </div>
      <div className="rowControl">
        {s.is_default ? (
          <span className="chip chipPrimary">Default</span>
        ) : (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setDefault.mutate(s.id)}>
            Make default
          </button>
        )}
        {many && (
          <>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Move ${s.name} up`} onClick={() => move.mutate({ id: s.id, up: true })}>
              ▲
            </button>
            <button type="button" className="btn btnGhost btnCompact" aria-label={`Move ${s.name} down`} onClick={() => move.mutate({ id: s.id, up: false })}>
              ▼
            </button>
          </>
        )}
        <span title={why ?? undefined}>
          <DangerButton confirm={`Delete “${s.name}”?`} disabled={!!why || remove.isPending} onConfirm={() => remove.mutate(s.id)}>
            Delete
          </DangerButton>
        </span>
      </div>
    </div>
  );
}

function AddState({ slug, projectId, group }: { slug: string; projectId: string; group: StateGroup }) {
  const create = usePmStateMutation(slug, projectId, stateCalls.create);
  const [name, setName] = useState("");
  const [color, setColor] = useState("gray");
  const add = () => name.trim() && create.mutate({ projectId, input: { name, description: "", color, group } }, { onSuccess: () => setName("") });
  return (
    <div className="groupRow">
      <div className={styles.stateMain}>
        <ColorPick label="New state's colour" value={color} onChange={setColor} />
        <input className="input" aria-label={`New ${group} state`} placeholder="Add a state" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && add()} />
      </div>
      <div className="rowControl">
        <button type="button" className="btn btnSecondary btnCompact" disabled={!name.trim() || create.isPending} onClick={add}>
          Add
        </button>
        {create.error && <span className="formError">{create.error.message}</span>}
      </div>
    </div>
  );
}

function States({ slug, projectId }: { slug: string; projectId: string }) {
  const states = usePmStates(slug, projectId).data ?? [];
  return (
    <>
      <p className={`fieldHint ${styles.hint}`}>Every work item is in a state. The five groups are fixed; each keeps at least one state, and one state is where new items start.</p>
      {GROUPS.map((g) => {
        const own = states.filter((s) => s.group === g.key);
        return (
          <div key={g.key}>
            <p className="groupTitle">{g.label}</p>
            <div className="group">
              {own.map((s) => (
                <StateRow key={s.id} slug={slug} projectId={projectId} state={s} many={own.length > 1} />
              ))}
              <AddState slug={slug} projectId={projectId} group={g.key} />
            </div>
          </div>
        );
      })}
    </>
  );
}

function LabelRow({ slug, projectId, label: l, groups, hasChildren }: { slug: string; projectId: string; label: PmLabel; groups: PmLabel[]; hasChildren: boolean }) {
  const update = usePmLabelMutation(slug, projectId, labelCalls.update);
  const remove = usePmLabelMutation(slug, projectId, labelCalls.remove);
  const put = (patch: Partial<{ name: string; description: string; color: string; parent_id: string | null }>) => update.mutate({ id: l.id, input: { name: l.name, description: l.description, color: l.color, parent_id: l.parent_id, ...patch } });
  return (
    <div className={`groupRow ${l.parent_id ? styles.child : ""}`}>
      <div className={styles.stateMain}>
        <ColorPick label={`${l.name} colour`} value={l.color} disabled={update.isPending} onChange={(color) => put({ color })} />
        <div className="rowText">
          <Text key={l.name} label={`${l.name} name`} value={l.name} onSave={(name) => put({ name })} />
          {(update.error ?? remove.error) && <p className="formError">{(update.error ?? remove.error)?.message}</p>}
        </div>
      </div>
      <div className="rowControl">
        {!hasChildren && (
          <select className="input" aria-label={`${l.name} group`} value={l.parent_id ?? ""} onChange={(e) => put({ parent_id: e.target.value || null })}>
            <option value="">No group</option>
            {groups
              .filter((g) => g.id !== l.id)
              .map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
          </select>
        )}
        <DangerButton confirm={hasChildren ? `Delete “${l.name}” and its labels?` : `Delete “${l.name}”?`} disabled={remove.isPending} onConfirm={() => remove.mutate(l.id)}>
          Delete
        </DangerButton>
      </div>
    </div>
  );
}

function Labels({ slug, projectId }: { slug: string; projectId: string }) {
  const labels = usePmLabels(slug, projectId).data ?? [];
  const create = usePmLabelMutation(slug, projectId, labelCalls.create);
  const [name, setName] = useState("");
  const [color, setColor] = useState("gray");
  const [parent, setParent] = useState("");
  const top = labels.filter((l) => !l.parent_id);
  const ordered = top.flatMap((t) => [t, ...labels.filter((l) => l.parent_id === t.id)]);
  const add = () => name.trim() && create.mutate({ projectId, input: { name, description: "", color, parent_id: parent || null } }, { onSuccess: () => setName("") });
  return (
    <>
      <p className={`fieldHint ${styles.hint}`}>Labels tag work items. A label can sit in a group (one level).</p>
      <div className="group">
        {ordered.map((l) => (
          <LabelRow key={l.id} slug={slug} projectId={projectId} label={l} groups={top} hasChildren={labels.some((c) => c.parent_id === l.id)} />
        ))}
        {!labels.length &&
          [0, 1].map((i) => (
            <div key={i} className="groupRow ghostCard">
              {i === 0 && <span className="ghostText">No labels yet</span>}
            </div>
          ))}
        <div className="groupRow">
          <div className={styles.stateMain}>
            <ColorPick label="New label's colour" value={color} onChange={setColor} />
            <input className="input" aria-label="New label" placeholder="Add a label" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && add()} />
          </div>
          <div className="rowControl">
            <select className="input" aria-label="New label's group" value={parent} onChange={(e) => setParent(e.target.value)}>
              <option value="">No group</option>
              {top.map((g) => (
                <option key={g.id} value={g.id}>
                  {g.name}
                </option>
              ))}
            </select>
            <button type="button" className="btn btnSecondary btnCompact" disabled={!name.trim() || create.isPending} onClick={add}>
              Add
            </button>
          </div>
        </div>
      </div>
      {create.error && (
        <p className="formError" role="alert">
          {create.error.message}
        </p>
      )}
    </>
  );
}

function Values({ values, onChange }: { values: string[]; onChange: (v: string[]) => void }) {
  return (
    <div className={styles.emojiRow}>
      {values.map((v, i) => (
        <input key={i} className={`input ${styles.valueInput}`} aria-label={`Value ${i + 1}`} value={v} onChange={(e) => onChange(values.map((x, j) => (j === i ? e.target.value : x)))} />
      ))}
      {values.length < 6 && (
        <button type="button" className="btn btnGhost btnCompact" onClick={() => onChange([...values, ""])}>
          + Value
        </button>
      )}
      {values.length > 1 && (
        <button type="button" className="btn btnGhost btnCompact" onClick={() => onChange(values.slice(0, -1))}>
          − Value
        </button>
      )}
    </div>
  );
}

function Estimates({ slug, projectId }: { slug: string; projectId: string }) {
  const estimates = usePmEstimates(slug, projectId).data ?? [];
  const create = usePmEstimateMutation(slug, projectId, estimateCalls.create);
  const update = usePmEstimateMutation(slug, projectId, estimateCalls.update);
  const disable = usePmEstimateMutation(slug, projectId, estimateCalls.disable);
  const active = estimates.find((e) => e.active);
  const [preset, setPreset] = useState(0);
  const [kind, setKind] = useState<EstimateKind>(ESTIMATE_PRESETS[0].kind);
  const [name, setName] = useState(ESTIMATE_PRESETS[0].name);
  const [values, setValues] = useState(ESTIMATE_PRESETS[0].values);
  const [editing, setEditing] = useState<string[] | null>(null);
  const pick = (i: number) => {
    setPreset(i);
    const p = ESTIMATE_PRESETS[i];
    if (p) {
      setKind(p.kind);
      setName(p.name);
      setValues(p.values);
    } else {
      setName("Custom");
      setValues(["", ""]);
    }
  };
  const error = create.error ?? update.error ?? disable.error;
  return (
    <>
      <p className={`fieldHint ${styles.hint}`}>An estimate system sizes work items: points or categories, up to six values. One is active; a new one archives it.</p>
      <p className="groupTitle">Active</p>
      <div className="group">
        {active ? (
          <div className="groupRow">
            <div className="rowText">
              <p className="rowLabel">
                {active.name} <span className="chip">{active.kind === "points" ? "Points" : "Categories"}</span>
              </p>
              {editing ? <Values values={editing} onChange={setEditing} /> : <p className="rowHint">{active.points.map((x) => x.value).join(" · ")}</p>}
            </div>
            <div className="rowControl">
              {editing ? (
                <>
                  <button type="button" className="btn btnGhost btnCompact" onClick={() => setEditing(null)}>
                    Cancel
                  </button>
                  <button type="button" className="btn btnSecondary btnCompact" disabled={update.isPending} onClick={() => update.mutate({ id: active.id, input: { name: active.name, kind: active.kind, values: editing } }, { onSuccess: () => setEditing(null) })}>
                    Save
                  </button>
                </>
              ) : (
                <>
                  <button type="button" className="btn btnSecondary btnCompact" onClick={() => setEditing(active.points.map((x) => x.value))}>
                    Edit values
                  </button>
                  <DangerButton confirm="Turn estimates off?" disabled={disable.isPending} onConfirm={() => disable.mutate(projectId)}>
                    Turn off
                  </DangerButton>
                </>
              )}
            </div>
          </div>
        ) : (
          [0, 1].map((i) => (
            <div key={i} className="groupRow ghostCard">
              {i === 0 && <span className="ghostText">No estimates — work items aren't sized</span>}
            </div>
          ))
        )}
      </div>
      <p className="groupTitle">New estimate system</p>
      <div className="group">
        <Row label="Start from">
          <select className="input" aria-label="Start from" value={preset} onChange={(e) => pick(Number(e.target.value))}>
            {ESTIMATE_PRESETS.map((p, i) => (
              <option key={p.name} value={i}>
                {p.name} ({p.kind === "points" ? "points" : "categories"})
              </option>
            ))}
            <option value={ESTIMATE_PRESETS.length}>Custom</option>
          </select>
        </Row>
        {preset === ESTIMATE_PRESETS.length && (
          <Row label="Kind">
            <div className="segmented" role="group" aria-label="Kind">
              <button type="button" aria-pressed={kind === "points"} onClick={() => setKind("points")}>
                Points
              </button>
              <button type="button" aria-pressed={kind === "categories"} onClick={() => setKind("categories")}>
                Categories
              </button>
            </div>
          </Row>
        )}
        <Row label="Name">
          <input className="input" aria-label="Estimate name" value={name} onChange={(e) => setName(e.target.value)} />
        </Row>
        <Row label="Values">
          <Values values={values} onChange={setValues} />
        </Row>
        <Row label="">
          <button type="button" className="btn btnSecondary btnCompact" disabled={create.isPending || !name.trim()} onClick={() => create.mutate({ projectId, input: { name, kind, values } }, { onSuccess: () => setEditing(null) })}>
            {active ? "Replace the active one" : "Use it"}
          </button>
        </Row>
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {estimates.some((e) => !e.active) && (
        <>
          <p className="groupTitle">Archived</p>
          <div className="group">
            {estimates
              .filter((e) => !e.active)
              .map((e) => (
                <div key={e.id} className="groupRow">
                  <div className="rowText">
                    <p className="rowLabel">{e.name}</p>
                    <p className="rowHint">{e.points.map((x) => x.value).join(" · ")}</p>
                  </div>
                </div>
              ))}
          </div>
        </>
      )}
    </>
  );
}

function Automations({ slug, project: p, busy, save }: { slug: string; project: PmProject; busy: boolean; save: (patch: PmProjectPatch) => void }) {
  const states = usePmStates(slug, p.id).data ?? [];
  const cancelled = states.filter((s) => s.group === "cancelled");
  const a = p.automations;
  const months = (n: number) => (n === 0 ? "Off" : n === 1 ? "After 1 month" : `After ${n} months`);
  return (
    <div className="group">
      <Row label="Auto-archive closed work items" hint="Completed or cancelled items are archived after this long.">
        <select className="input" aria-label="Auto-archive" value={a.archive_in} disabled={busy} onChange={(e) => save({ automations: { ...a, archive_in: Number(e.target.value) } })}>
          {MONTHS.map((m) => (
            <option key={m} value={m}>
              {months(m)}
            </option>
          ))}
        </select>
      </Row>
      <Row label="Auto-close idle work items" hint="Items untouched this long move to a cancelled state.">
        <select className="input" aria-label="Auto-close" value={a.close_in} disabled={busy} onChange={(e) => save({ automations: { ...a, close_in: Number(e.target.value), close_state_id: a.close_state_id ?? cancelled[0]?.id ?? null } })}>
          {MONTHS.map((m) => (
            <option key={m} value={m}>
              {months(m)}
            </option>
          ))}
        </select>
      </Row>
      {a.close_in > 0 && (
        <Row label="Close into">
          <select className="input" aria-label="Close into" value={a.close_state_id ?? ""} disabled={busy} onChange={(e) => save({ automations: { ...a, close_state_id: e.target.value || null } })}>
            {cancelled.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </Row>
      )}
      <Row label="Due-date reminders" hint="Your inbox reminds you of items due, the day before.">
        <OnOff label="Due-date reminders" value={a.due_reminders} disabled={busy} onChange={(on) => save({ automations: { ...a, due_reminders: on } })} />
      </Row>
    </div>
  );
}
