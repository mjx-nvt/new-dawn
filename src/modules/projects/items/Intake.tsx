import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import type { PmProject } from "../../../kernel/bridge/pm";
import { useIntakeMutations, usePmIntake, type IntakeStatus, type PmIntakeItem } from "../../../kernel/bridge/pmIntake";
import { itemFiles, useSearchPmItems, useUpdatePmItem, type ItemPatch, type Priority } from "../../../kernel/bridge/pmItems";
import Editor from "../../../kernel/editor/Editor";
import { dayString, formatDateTime, formatDay } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import { inField } from "../../../kernel/shell/useShortcuts";
import ItemPeek, { type PeekMode } from "./ItemPeek";
import { Activity, type Tab } from "./ItemView";
import { DateField, LabelPick, PeoplePick, PriorityMark, PriorityPick, StatePick } from "./Props";
import { PRIORITIES, PRIORITY_RANK } from "./items";
import styles from "./Items.module.css";

const route = getRouteApi("/shell/projects/$projectId");
const SORTS = [
  { key: "newest", label: "Newest" },
  { key: "oldest", label: "Oldest" },
  { key: "priority", label: "Priority" },
  { key: "due", label: "Due date" },
] as const;
const STATUS_LABEL: Record<IntakeStatus, string> = { pending: "Pending", accepted: "Accepted", declined: "Declined", snoozed: "Snoozed", duplicate: "Duplicate" };
/** Can still be acted on (a snoozed one too). */
const open = (x: PmIntakeItem) => x.status === "pending" || x.status === "snoozed";
/** Waits in the Open tab: pending (a snooze whose day has come reads pending); a snoozed one is
 *  in Closed until its day, as Plane has it. */
const waiting = (x: PmIntakeItem) => x.status === "pending";

function statusText(x: PmIntakeItem): string {
  if (x.status === "snoozed" && x.snoozed_until) return `Snoozed until ${formatDay(x.snoozed_until, { short: true })}`;
  if (x.status === "duplicate") return x.duplicate_key ? `Duplicate of ${x.duplicate_key}` : "Duplicate";
  return STATUS_LABEL[x.status];
}

/** A request by hand (Plane's intake form): a title, a description, a priority, assignees,
 *  labels, a due date — it waits in Intake. */
function IntakeSheet({ slug, project, onClose, onMade }: { slug: string; project: PmProject; onClose: () => void; onMade: (id: string) => void }) {
  const m = useIntakeMutations(slug);
  const dialog = useRef<HTMLDialogElement>(null);
  const [name, setName] = useState("");
  const [text, setText] = useState("");
  const [priority, setPriority] = useState<Priority>("none");
  const [assignees, setAssignees] = useState<string[]>([]);
  const [labels, setLabels] = useState<string[]>([]);
  const [due, setDue] = useState("");
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  const save = () => {
    if (!name.trim()) return;
    // The description as the editor keeps it: a paragraph a line.
    const lines = text.split("\n").filter((l) => l.trim());
    const description = lines.length ? JSON.stringify({ type: "doc", content: lines.map((l) => ({ type: "paragraph", content: [{ type: "text", text: l }] })) }) : null;
    m.create.mutate({ project_id: project.id, name, description, priority, assignees, labels, target_date: due || null }, { onSuccess: (made) => (onMade(made.item.id), onClose()) });
  };
  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.create}`}
      aria-labelledby="intake-sheet-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div className="sheetBody">
        <h2 id="intake-sheet-title" className="sheetHeading">
          New request
        </h2>
        <label className="field">
          <span className="fieldLabel">Title</span>
          <input className="input" autoFocus maxLength={255} placeholder="What's asked" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && save()} />
        </label>
        <label className="field">
          <span className="fieldLabel">Description</span>
          <textarea className="input" rows={4} placeholder="Who asks, what for, by when" value={text} onChange={(e) => setText(e.target.value)} />
        </label>
        <div className={styles.props}>
          <PriorityPick value={priority} onChange={setPriority} />
          <PeoplePick slug={slug} label="Assignees" value={assignees} onChange={setAssignees} />
          <LabelPick slug={slug} projectId={project.id} value={labels} onChange={setLabels} />
          <label className={styles.date}>
            <span className="fieldLabel">Due</span>
            <input className="input" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
          </label>
        </div>
        {m.create.error && (
          <p className="formError" role="alert">
            {m.create.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btnPrimary" disabled={!name.trim() || m.create.isPending} onClick={save}>
            Add request
          </button>
        </div>
      </div>
    </dialog>
  );
}

/** One request (D-081): what it asks — title, description, priority, assignees, labels, due — its
 *  comments and activity, and what to do with it while it is open: Accept into a state, Decline,
 *  Snooze until a day, Mark as a duplicate of a work item, Delete. */
function IntakeDetail({ slug, project, request: x, onPeek, onGone }: { slug: string; project: PmProject; request: PmIntakeItem; onPeek: (id: string) => void; onGone: () => void }) {
  const m = useIntakeMutations(slug);
  const update = useUpdatePmItem(slug);
  const i = x.item;
  const [name, setName] = useState(i.name);
  const [state, setState] = useState<string | null>(null);
  const [until, setUntil] = useState("");
  const [words, setWords] = useState("");
  const [tab, setTab] = useState<Tab>("all");
  const searching = words.trim().length > 1;
  const found = useSearchPmItems(slug, words, i.id, searching, project.id);
  const ro = !!project.archived_at || !open(x);
  const save = (patch: ItemPatch) => update.mutate({ id: i.id, patch });
  const error = m.accept.error ?? m.decline.error ?? m.snooze.error ?? m.duplicate.error ?? m.remove.error ?? update.error;
  // The work items of the project it may repeat (none until two letters are typed).
  const candidates = searching ? (found.data ?? []).slice(0, 6) : [];
  const tomorrow = (() => {
    const d = new Date();
    return dayString(new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1));
  })();
  return (
    <article className={styles.view} aria-label={`${i.key} ${i.name}`}>
      <header className={styles.viewHead}>
        <span className={styles.key}>{i.key}</span>
        <span className={`chip ${x.status === "pending" ? "chipPrimary" : ""}`}>{statusText(x)}</span>
        {x.source === "email" && (
          <span className="fieldHint">
            From an email{x.email_subject ? `: “${x.email_subject}”` : ""}
            {x.email_sender ? ` — ${x.email_sender}` : ""}
          </span>
        )}
        <span className={styles.spacer} />
        {x.status === "accepted" && (
          <button type="button" className="btn btnSecondary btnCompact" onClick={() => onPeek(i.id)}>
            Open the work item
          </button>
        )}
        {x.status !== "accepted" && !project.archived_at && (
          <DangerButton confirm={`Delete ${i.key}? Its comments go with it.`} onConfirm={() => m.remove.mutateAsync(i.id).then(onGone, () => undefined)}>
            Delete
          </DangerButton>
        )}
      </header>
      <input className={styles.title} aria-label="Title" value={name} disabled={ro} onChange={(e) => setName(e.target.value)} onBlur={() => (!name.trim() ? setName(i.name) : name !== i.name && save({ name }))} onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()} />
      {open(x) && !project.archived_at && (
        <div className={`glass ${styles.intakeActions}`} role="group" aria-label="What to do with it">
          <div className={styles.inline}>
            <StatePick slug={slug} projectId={project.id} value={state} onChange={(id) => setState(id || null)} />
            <button type="button" className="btn btnPrimary btnCompact" disabled={m.accept.isPending} onClick={() => m.accept.mutate({ id: i.id, stateId: state })}>
              Accept
            </button>
            <button type="button" className="btn btnSecondary btnCompact" disabled={m.decline.isPending} onClick={() => m.decline.mutate(i.id)}>
              Decline
            </button>
          </div>
          <div className={styles.inline}>
            <label className={styles.date}>
              <span className="fieldLabel">Snooze until</span>
              <input className="input" type="date" min={tomorrow} value={until} onChange={(e) => setUntil(e.target.value)} />
            </label>
            <button type="button" className="btn btnSecondary btnCompact" disabled={!until || m.snooze.isPending} onClick={() => m.snooze.mutate({ id: i.id, until })}>
              Snooze
            </button>
          </div>
          <div className={styles.inline}>
            <input className={`input ${styles.search}`} aria-label="Find the work item it repeats" placeholder="Duplicate of… (a key or words)" value={words} onChange={(e) => setWords(e.target.value)} />
            {candidates.map((r) => (
              <button key={r.id} type="button" className="btn btnGhost btnCompact" aria-label={`Mark as a duplicate of ${r.key} ${r.name}`} disabled={m.duplicate.isPending} onClick={() => m.duplicate.mutate({ id: i.id, of: r.id })}>
                {r.key} {r.name}
              </button>
            ))}
          </div>
        </div>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      {ro && !i.description ? (
        <p className="ghostText">Not yet captured</p>
      ) : (
        <Editor key={`${i.id}-${ro}`} value={i.description ?? null} readOnly={ro} placeholder="Add a description…" home={{ slug, object: "pm-items", recordId: i.id, files: itemFiles(slug, i.id) }} onChange={(json) => save({ description: json })} />
      )}
      <dl className={styles.intakeProps}>
        <dt>Priority</dt>
        <dd>
          <PriorityPick value={i.priority} disabled={ro} onChange={(priority) => save({ priority })} />
        </dd>
        <dt>Assignees</dt>
        <dd>
          <PeoplePick slug={slug} label="Assignees" value={i.assignees} disabled={ro} onChange={(assignees) => save({ assignees })} />
        </dd>
        <dt>Labels</dt>
        <dd>
          <LabelPick slug={slug} projectId={project.id} value={i.labels} disabled={ro} onChange={(labels) => save({ labels })} />
        </dd>
        <dt>Due date</dt>
        <dd>
          <DateField label={`Due date of ${i.key}`} value={i.target_date} disabled={ro} onSave={(target_date) => save({ target_date })} />
        </dd>
        <dt>Received</dt>
        <dd className="fieldHint">{formatDateTime(i.created_at)}</dd>
      </dl>
      <Activity slug={slug} item={i} tab={tab} onTab={setTab} readOnly={ro} />
    </article>
  );
}

/** A project's Intake tab (D-081, Plane's): Open and Closed (in the URL), a sort and a priority
 *  filter, the requests on the left — ↑ and ↓ (or J and K) go through them — the one chosen on
 *  the right; New request (`N`). */
export default function Intake({ slug, project }: { slug: string; project: PmProject }) {
  const search = route.useSearch();
  const navigate = useNavigate();
  const intake = usePmIntake(slug, project.id);
  const [making, setMaking] = useState(false);
  const set = (patch: Record<string, string | undefined>) => void navigate({ to: "/projects/$projectId", params: { projectId: project.id }, search: { ...search, ...patch }, replace: true });
  const closedTab = search.itab === "closed";
  const sort = SORTS.find((s) => s.key === search.isort)?.key ?? "newest";
  const all = intake.data ?? [];
  const shown = all
    .filter((x) => (closedTab ? !waiting(x) : waiting(x)) && (!search.iprio || x.item.priority === search.iprio))
    .sort((a, b) => {
      if (sort === "oldest") return a.item.created_at.localeCompare(b.item.created_at);
      if (sort === "priority") return PRIORITY_RANK[a.item.priority] - PRIORITY_RANK[b.item.priority];
      if (sort === "due") return (a.item.target_date ?? "9999").localeCompare(b.item.target_date ?? "9999");
      return b.item.created_at.localeCompare(a.item.created_at);
    });
  const chosen = shown.find((x) => x.item.id === search.intake) ?? shown[0];
  const ro = !!project.archived_at;
  const openCount = all.filter(waiting).length;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.metaKey || e.ctrlKey || e.altKey || making || search.peek || inField(e.target) || document.querySelector("dialog[open]")) return;
      const at = shown.findIndex((x) => x.item.id === chosen?.item.id);
      if ((e.key === "ArrowDown" || e.key === "j") && at < shown.length - 1) {
        e.preventDefault();
        set({ intake: shown[at + 1].item.id });
      } else if ((e.key === "ArrowUp" || e.key === "k") && at > 0) {
        e.preventDefault();
        set({ intake: shown[at - 1].item.id });
      } else if (e.key === "n" && !ro) {
        e.preventDefault();
        setMaking(true);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <section className="section" aria-label="Intake">
      <div className={styles.toolbar}>
        <div className="segmented" role="tablist" aria-label="Intake">
          <button type="button" role="tab" aria-selected={!closedTab} onClick={() => set({ itab: undefined, intake: undefined })}>
            Open · {openCount}
          </button>
          <button type="button" role="tab" aria-selected={closedTab} onClick={() => set({ itab: "closed", intake: undefined })}>
            Closed · {all.length - openCount}
          </button>
        </div>
        <select className="input" aria-label="Sort by" value={sort} onChange={(e) => set({ isort: e.target.value === "newest" ? undefined : e.target.value })}>
          {SORTS.map((s) => (
            <option key={s.key} value={s.key}>
              {s.label}
            </option>
          ))}
        </select>
        <select className="input" aria-label="Priority" value={search.iprio ?? ""} onChange={(e) => set({ iprio: e.target.value || undefined })}>
          <option value="">Any priority</option>
          {PRIORITIES.map((p) => (
            <option key={p.key} value={p.key}>
              {p.label}
            </option>
          ))}
        </select>
        <span className={styles.spacer} />
        {/* One primary at a time: Accept while an open request is shown. */}
        <button type="button" className={chosen && open(chosen) && !ro ? "btn btnSecondary" : "btn btnPrimary"} disabled={ro} onClick={() => setMaking(true)}>
          New request
        </button>
      </div>
      {intake.error && <p className="formError">{intake.error.message}</p>}
      <div className={styles.intake}>
        <ul className={styles.intakeList} aria-label={closedTab ? "Closed requests" : "Open requests"}>
          {shown.map((x) => (
            <li key={x.item.id}>
              <button type="button" className={styles.intakeRow} aria-current={chosen?.item.id === x.item.id} onClick={() => set({ intake: x.item.id })}>
                <span className={styles.intakeRowHead}>
                  <span className={styles.key}>{x.item.key}</span>
                  <PriorityMark value={x.item.priority} />
                  <span className={styles.spacer} />
                  <span className="fieldHint">{formatDay(dayString(new Date(x.item.created_at)), { short: true })}</span>
                </span>
                <span className={styles.intakeRowTitle}>{x.item.name}</span>
                <span className={styles.chips}>
                  {(x.status !== "pending" || closedTab) && <span className="chip">{statusText(x)}</span>}
                  {x.source === "email" && <span className="chip">From email</span>}
                </span>
              </button>
            </li>
          ))}
          {!shown.length &&
            [0, 1, 2].map((n) => (
              <li key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">{intake.isError ? "Couldn't be read" : intake.data ? (closedTab ? "Nothing closed yet" : ro ? "No requests" : "No requests waiting — press N to add one") : "Reading…"}</span>}
              </li>
            ))}
        </ul>
        <div className={styles.intakeDetail}>
          {chosen ? (
            <IntakeDetail key={chosen.item.id} slug={slug} project={project} request={chosen} onPeek={(id) => set({ peek: id })} onGone={() => set({ intake: undefined })} />
          ) : (
            [0, 1].map((n) => (
              <div key={n} className={`${styles.ghostRow} ghostCard`}>
                {n === 0 && <span className="ghostText">Choose a request</span>}
              </div>
            ))
          )}
        </div>
      </div>
      {making && <IntakeSheet slug={slug} project={project} onClose={() => setMaking(false)} onMade={(id) => set({ itab: undefined, intake: id })} />}
      {search.peek && (
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
    </section>
  );
}
