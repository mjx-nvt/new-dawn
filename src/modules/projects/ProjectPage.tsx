import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { useArchivePmProject, usePmProject } from "../../kernel/bridge/pm";
import { useVisitPm } from "../../kernel/bridge/pmHome";
import { useDeleteProject, useProject, useUpdateProject, type Project, type ProjectPatch, type ProjectUpdate } from "../../kernel/bridge/projects";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import MenuButton from "../../kernel/shell/MenuButton";
import { usePageTitle } from "../../kernel/shell/pageTitle";
import { formatWhen } from "../meetings/format";
import { FIELD_LABEL, STATUS_LABEL, STATUSES, formatDay, milestoneText } from "./format";
import Archived from "./items/Archived";
import Cycles from "./items/Cycles";
import ImportSheet from "./items/ImportSheet";
import Intake from "./items/Intake";
import Modules from "./items/Modules";
import Pages from "./items/Pages";
import Views from "./items/Views";
import WorkItems from "./items/WorkItems";
import ProjectSettings from "./ProjectSettings";
import Proposals from "./Proposals";
import styles from "./Projects.module.css";

const route = getRouteApi("/shell/projects/$projectId");

/** An IME is still writing (Vietnamese): Enter confirms the composition, not the field. */
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

/** A project's page (D-054, D-073): its Overview — the updates its meetings brought, where it
 *  stands, its risks, decisions, meetings and history (every section shown even when empty) —
 *  beside Plane's work items, cycles, modules, views, pages and intake (as its features say),
 *  archives and settings — each a row of the Projects rail; the part (`tab`) is URL state. */
export default function ProjectPage() {
  const { projectId } = route.useParams();
  const { tab: wanted, section } = route.useSearch();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const navigate = useNavigate();
  const dossier = useProject(slug, projectId);
  const plane = usePmProject(slug, projectId);
  const pm = plane.data;
  const archive = useArchivePmProject(slug);
  const f = pm?.features;
  useVisitPm(pm ? slug : undefined, "project", projectId);
  // The project's parts are the rail's rows (D-089, D-090); `parts` says which this project has.
  const parts = ["overview", "items", ...(f?.cycles ? ["cycles"] : []), ...(f?.modules ? ["modules"] : []), ...(f?.views ? ["views"] : []), ...(f?.pages ? ["pages"] : []), ...(f?.intake ? ["intake"] : []), "archives", "settings"];
  const tab = parts.find((t) => t === wanted) ?? "overview";
  // A tab of its own: the list's group, order and peek are the Work items tab's.
  const showTab = (t: string) => void navigate({ to: "/projects/$projectId", params: { projectId }, search: t === "overview" ? {} : { tab: t }, replace: true });
  const update = useUpdateProject(slug);
  const remove = useDeleteProject(slug);
  const [editing, setEditing] = useState(false);
  const [importing, setImporting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const d = dossier.data;
  const p = d?.project;
  usePageTitle(p?.name);
  const save = (patch: ProjectPatch) => p && update.mutate({ id: p.id, patch });
  const error = dossier.error ?? plane.error ?? update.error ?? remove.error ?? archive.error;

  return (
    <div className="page">
      <div className={styles.header}>
        <div className={styles.titles}>
          <p className="eyebrow">
            Project{pm ? ` · ${pm.identifier}` : ""}
            {pm?.archived_at ? " · Archived" : ""}
          </p>
          <h1 className={`pageTitle ${styles.ptitle}`}>
            {pm?.emoji && <span aria-hidden>{pm.emoji}</span>}
            {p?.name ?? "…"}
          </h1>
          {p && <p className={styles.meta}>{[p.phase, milestoneText(p)].filter(Boolean).join(" · ") || "No phase or milestone yet"}</p>}
        </div>
        {p && (
          <div className={styles.headActions}>
            <div className="segmented" role="group" aria-label="Status">
              {STATUSES.map((s) => (
                <button key={s} type="button" aria-pressed={p.status === s} disabled={update.isPending} onClick={() => save({ status: s })}>
                  {STATUS_LABEL[s]}
                </button>
              ))}
            </div>
            <MenuButton
              className="btn btnGhost"
              items={[
                { label: "Edit…", onSelect: () => setEditing(true) },
                { label: "Settings", onSelect: () => showTab("settings") },
                ...(pm && !pm.archived_at ? [{ label: "Import work items (Plane CSV)…", onSelect: () => setImporting(true) }] : []),
                ...(pm ? [{ label: pm.archived_at ? "Restore project" : "Archive project", separator: true, onSelect: () => archive.mutate({ id: pm.id, archived: !pm.archived_at }) }] : []),
                { label: "Delete project…", danger: true, separator: true, onSelect: () => setConfirmDelete(true) },
              ]}
            />
          </div>
        )}
      </div>
      {error && !editing && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {confirmDelete && p && (
        <div className={`card ${styles.confirm}`} role="alertdialog" aria-labelledby="delete-project-title">
          <div>
            <p id="delete-project-title" className={styles.confirmTitle}>
              Delete “{p.name}”?
            </p>
            <p className="fieldHint">Its risks, decisions, notes, history and waiting updates go with it, and its Plane side — states, labels, estimates, work items, cycles, modules and pages. Its meetings stay.</p>
          </div>
          <div className={styles.confirmActions}>
            <button type="button" className="btn btnGhost btnCompact" autoFocus onClick={() => setConfirmDelete(false)}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btnDangerFilled btnCompact"
              disabled={remove.isPending}
              onClick={() => remove.mutate(p.id, { onSuccess: () => void navigate({ to: "/projects", search: {} }) })}
            >
              Delete project
            </button>
          </div>
        </div>
      )}

      {tab === "settings" && pm && slug && <ProjectSettings slug={slug} project={pm} section={section} />}
      {tab === "items" && pm && slug && <WorkItems slug={slug} project={pm} />}
      {tab === "archives" && pm && slug && <Archived slug={slug} projectId={pm.id} />}
      {tab === "views" && pm && slug && <Views slug={slug} project={pm} />}
      {tab === "cycles" && pm && slug && <Cycles slug={slug} project={pm} />}
      {tab === "modules" && pm && slug && <Modules slug={slug} project={pm} />}
      {tab === "pages" && pm && slug && <Pages slug={slug} project={pm} />}
      {tab === "intake" && pm && slug && <Intake slug={slug} project={pm} />}
      {tab === "overview" && (
      <>
      <section className="section">
        <h2 className="sectionTitle">Updates waiting for you</h2>
        <p className="sectionSubtitle">Proposed from its meetings and the emails you read — nothing changes until you apply one</p>
        <Waiting slug={slug} updates={d?.waiting} />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Where it stands</h2>
        <div className={styles.fields}>
          <Field label="Phase" value={p?.phase} />
          <Field label="Next milestone" value={p ? milestoneText(p) : undefined} />
          <Field label="Notes" value={p?.notes} />
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Risks</h2>
        <Items items={p?.risks} noun="risk" busy={update.isPending} onChange={(risks) => save({ risks })} />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Decisions</h2>
        <Items items={p?.decisions} noun="decision" busy={update.isPending} onChange={(decisions) => save({ decisions })} />
      </section>

      <section className="section">
        <h2 className="sectionTitle">Meetings</h2>
        {d?.meetings.length ? (
          <ul className={styles.items}>
            {d.meetings.map((m) => (
              <li key={m.id} className={styles.item}>
                <Link to="/meetings/$meetingId" params={{ meetingId: m.id }} search={{ tab: "summary" }} className={styles.link}>
                  {m.title}
                </Link>
                <span className="fieldHint">{formatWhen(m.held_on)}</span>
              </li>
            ))}
          </ul>
        ) : (
          <Ghosts text="Not yet captured — the meetings that touch it show here" />
        )}
      </section>

      <section className="section">
        <h2 className="sectionTitle">History</h2>
        {d?.history.length ? (
          <ul className={styles.items}>
            {d.history.map((u) => (
              <li key={u.id} className={styles.item}>
                <span>
                  <span className="fieldLabel">{FIELD_LABEL[u.field]}</span> {u.field === "item_state" || u.field === "comment" ? `${u.item_key ?? "A work item no longer kept"} — ` : ""}
                  {u.value}
                  {u.due ? ` · by ${formatDay(u.due)}` : ""}
                </span>
                <span className="fieldHint">
                  {u.email_item_id ? `Email “${u.email_subject || "No subject"}” · ` : u.meeting_title ? `${u.meeting_title} · ` : ""}applied {formatWhen(u.updated_at)}
                </span>
              </li>
            ))}
          </ul>
        ) : (
          <Ghosts text="Not yet captured — what you apply is kept here" />
        )}
      </section>
      </>
      )}

      {importing && pm && slug && <ImportSheet slug={slug} projectId={pm.id} projectName={pm.name} onClose={() => setImporting(false)} />}
      {editing && p && (
        <EditSheet
          project={p}
          saving={update.isPending}
          error={update.error?.message ?? null}
          onSave={(patch) => update.mutate({ id: p.id, patch }, { onSuccess: () => setEditing(false) })}
          onClose={() => {
            update.reset();
            setEditing(false);
          }}
        />
      )}
    </div>
  );
}

function Ghosts({ text }: { text: string }) {
  return (
    <div className={styles.list}>
      <p className="ghostText">{text}</p>
      {[0, 1].map((i) => (
        <div key={i} className={`${styles.ghostRow} ghostCard`} />
      ))}
    </div>
  );
}

function Field({ label, value }: { label: string; value?: string }) {
  return (
    <div className={styles.field}>
      <p className="fieldLabel">{label}</p>
      {value ? <p className={styles.fieldValue}>{value}</p> : <p className="fieldMuted">Not yet captured</p>}
    </div>
  );
}

/** The proposals waiting, or ghost rows saying where they come from. */
function Waiting({ slug, updates }: { slug?: string; updates?: ProjectUpdate[] }) {
  if (!updates?.length) return <Ghosts text="Not yet captured — after each summary, the changes its meetings bring show here" />;
  return <Proposals slug={slug} updates={updates} />;
}

/** Risks or decisions: each with Remove, and a field to add one (Enter). */
function Items({ items, noun, busy, onChange }: { items?: string[]; noun: string; busy: boolean; onChange: (items: string[]) => void }) {
  const [draft, setDraft] = useState("");
  const list = items ?? [];
  return (
    <div className={styles.list}>
      {list.length ? (
        <ul className={styles.items}>
          {list.map((item, i) => (
            <li key={`${i}-${item}`} className={styles.item}>
              <span>{item}</span>
              <button type="button" className="btn btnGhost btnCompact" disabled={busy} aria-label={`Remove the ${noun} “${item}”`} onClick={() => onChange(list.filter((_, k) => k !== i))}>
                Remove
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <>
          <p className="ghostText">Not yet captured</p>
          {[0, 1].map((i) => (
            <div key={i} className={`${styles.ghostRow} ghostCard`} />
          ))}
        </>
      )}
      <input
        className={`input ${styles.add}`}
        value={draft}
        placeholder={`Add a ${noun}`}
        aria-label={`Add a ${noun}`}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key !== "Enter" || composing(e) || !draft.trim() || busy) return;
          onChange([...list, draft.trim()]);
          setDraft("");
        }}
      />
    </div>
  );
}

/** Edit the name, phase, next milestone and its date, and the notes. */
function EditSheet({
  project: p,
  saving,
  error,
  onSave,
  onClose,
}: {
  project: Project;
  saving: boolean;
  error: string | null;
  onSave: (patch: ProjectPatch) => void;
  onClose: () => void;
}) {
  const [name, setName] = useState(p.name);
  const [phase, setPhase] = useState(p.phase);
  const [milestone, setMilestone] = useState(p.next_milestone);
  const [due, setDue] = useState(p.milestone_due ?? "");
  const [notes, setNotes] = useState(p.notes);
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="edit-project-title"
      onCancel={(e) => {
        e.preventDefault();
        if (!saving) onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && !saving && onClose()}
    >
      <div className="sheetBody">
        <h2 id="edit-project-title" className="sheetHeading">
          Edit project
        </h2>
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Phase</span>
          <input className="input" value={phase} onChange={(e) => setPhase(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Next milestone</span>
          <input className="input" value={milestone} onChange={(e) => setMilestone(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Milestone date</span>
          <input className="input" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Notes</span>
          <textarea className={`input ${styles.notes}`} rows={4} value={notes} onChange={(e) => setNotes(e.target.value)} />
        </label>
        {error && <p className="formError">{error}</p>}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={saving} onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={!name.trim() || saving}
            onClick={() => onSave({ name, phase, next_milestone: milestone, milestone_due: due, notes, status: null, risks: null, decisions: null })}
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
