import { useEffect, useRef, useState } from "react";
import { SparkleIcon } from "../../kernel/shell/icons";
import type { Attendee_Serialize } from "../../bindings";
import { useUpdateAttendees } from "../../kernel/bridge/people";
import { useSummarise } from "../../kernel/bridge/summary";
import styles from "./AttendeesBlock.module.css";

interface Row {
  name: string;
  title: string;
  organisation: string;
  attendance: string;
  /** The spelling the row had when editing began (kept as "also heard as" if changed). */
  heardAs: string | null;
  /** The row as it was (null = added now): only added or changed rows teach the memory. */
  original: Attendee_Serialize | null;
}

const toRow = (a: Attendee_Serialize): Row => ({ ...a, heardAs: a.name || null, original: a });
const blank = (): Row => ({ name: "", title: "", organisation: "", attendance: "Present", heardAs: null, original: null });

const changed = (r: Row) =>
  !r.original ||
  r.name !== r.original.name ||
  r.title !== r.original.title ||
  r.organisation !== r.original.organisation ||
  r.attendance !== r.original.attendance;

/** Who attended. Edit opens a sheet; saving fixes the names across the summary, email and
 *  minutes and (with Remember) teaches the workspace's people memory. */
export default function AttendeesBlock({
  slug,
  meetingId,
  attendees,
  locked,
}: {
  slug: string;
  meetingId: string;
  attendees: Attendee_Serialize[];
  locked: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [saved, setSaved] = useState(false);
  const [inline, setInline] = useState<number | null>(null);
  const summarise = useSummarise();
  const update = useUpdateAttendees(slug, meetingId);

  // Click a name: edit it in place; Enter saves (and remembers), Esc leaves it.
  const saveInline = (i: number, row: Row) => {
    const list = attendees.map((a, j) => {
      const r = j === i ? row : toRow(a);
      return { name: r.name, title: r.title, organisation: r.organisation, attendance: r.attendance, heard_as: r.heardAs, changed: changed(r) };
    });
    update.mutate({ attendees: list.filter((a) => a.name.trim()), remember: true }, { onSuccess: () => { setInline(null); setSaved(true); } });
  };

  return (
    <div className={styles.block}>
      <div className={styles.head}>
        <h3 className="sectionTitle">Attendees</h3>
        <button type="button" className="btn btnGhost btnCompact" disabled={locked} onClick={() => setEditing(true)}>
          Edit
        </button>
      </div>

      {attendees.length ? (
        <ul className={styles.list}>
          {attendees.map((a, i) =>
            inline === i ? (
              <li key={i} className={styles.inline}>
                <InlineEditor row={toRow(a)} saving={update.isPending} onCancel={() => setInline(null)} onSave={(row) => saveInline(i, row)} />
              </li>
            ) : (
              <li key={i}>
                <button type="button" className={styles.name} disabled={locked} title="Edit in place" onClick={() => setInline(i)}>
                  {a.name}
                </button>
                {[a.title, a.organisation].filter(Boolean).length > 0 && <span className={styles.role}>{[a.title, a.organisation].filter(Boolean).join(", ")}</span>}
                {a.attendance && a.attendance !== "Present" && <span className="chip">{a.attendance}</span>}
              </li>
            ),
          )}
        </ul>
      ) : (
        <>
          <p className="ghostText">Not yet captured</p>
          <div className={`${styles.slot} ghostCard`} />
          <div className={`${styles.slot} ghostCard`} />
        </>
      )}

      {saved && (
        <div className={styles.afterSave} role="status">
          <span className="fieldHint">Names fixed across the summary, email and minutes.</span>
          <button
            type="button"
            className="btn btnCompact btnAi"
            aria-busy={summarise.isPending || undefined}
            disabled={locked || summarise.isPending}
            title="Qwen rewrites the summary with the corrected people; your edited sections are kept"
            onClick={() => {
              setSaved(false);
              summarise.mutate({ slug, meetingId, deep: false, keepEdits: true });
            }}
          >
            <SparkleIcon />
            Re-run with corrected names
          </button>
          <button type="button" className="btn btnGhost btnCompact" aria-label="Dismiss" onClick={() => setSaved(false)}>
            OK
          </button>
        </div>
      )}

      {editing && (
        <AttendeesSheet
          slug={slug}
          meetingId={meetingId}
          attendees={attendees}
          onClose={() => setEditing(false)}
          onSaved={() => {
            setEditing(false);
            setSaved(true);
          }}
        />
      )}
    </div>
  );
}

/** One attendee edited where it stands: name, title, organisation; Enter saves, Esc cancels. */
function InlineEditor({ row, saving, onCancel, onSave }: { row: Row; saving: boolean; onCancel: () => void; onSave: (row: Row) => void }) {
  const [draft, setDraft] = useState(row);
  const keys = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") onCancel();
    else if (e.key === "Enter" && !e.nativeEvent.isComposing && draft.name.trim()) onSave(draft);
    else return;
    e.preventDefault();
  };
  return (
    <div className={styles.inlineFields}>
      <input className="input" autoFocus aria-label="Name" placeholder="Name" value={draft.name} onKeyDown={keys} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
      <input className="input" aria-label="Job title" placeholder="Job title" value={draft.title} onKeyDown={keys} onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
      <input className="input" aria-label="Organisation" placeholder="Organisation" value={draft.organisation} onKeyDown={keys} onChange={(e) => setDraft({ ...draft, organisation: e.target.value })} />
      <div className={styles.inlineActions}>
        <span className="fieldHint">Enter saves and remembers · Esc cancels</span>
        <button type="button" className="btn btnGhost btnCompact" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={saving || !draft.name.trim()} onClick={() => onSave(draft)}>
          {saving ? "Saving…" : "Save"}
        </button>
      </div>
    </div>
  );
}

/** The editor: a full-width table in a modal sheet (the side column is too narrow for one). */
function AttendeesSheet({
  slug,
  meetingId,
  attendees,
  onClose,
  onSaved,
}: {
  slug: string;
  meetingId: string;
  attendees: Attendee_Serialize[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const update = useUpdateAttendees(slug, meetingId);
  const [rows, setRows] = useState<Row[]>(() => (attendees.length ? attendees.map(toRow) : [blank()]));
  const [remember, setRemember] = useState(true);
  const set = (i: number, patch: Partial<Row>) => setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  // Closing the native dialog (not just unmounting) returns focus to the Edit button.
  const close = () => !update.isPending && dialog.current?.close();

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  return (
    <dialog
      ref={dialog}
      className={`glass sheet ${styles.sheet}`}
      aria-labelledby="attendees-title"
      onClose={onClose}
      onCancel={(e) => update.isPending && e.preventDefault()}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <div>
          <h2 id="attendees-title" className="sheetHeading">
            Attendees
          </h2>
          <p className="fieldHint">Fix a name once: it's corrected everywhere and remembered.</p>
        </div>

        <div className={styles.table}>
          <div className={styles.tableHead} aria-hidden>
            <span className="fieldLabel">Name</span>
            <span className="fieldLabel">Job title</span>
            <span className="fieldLabel">Organisation</span>
            <span className="fieldLabel">Attendance</span>
            <span />
          </div>
          {rows.map((r, i) => (
            <div key={i} className={styles.tableRow}>
              <input className="input" aria-label="Name" placeholder="Name" value={r.name} autoFocus={i === rows.length - 1 && !r.original} onChange={(e) => set(i, { name: e.target.value })} />
              <input className="input" aria-label="Job title" placeholder="Job title" value={r.title} onChange={(e) => set(i, { title: e.target.value })} />
              <input className="input" aria-label="Organisation" placeholder="Organisation" value={r.organisation} onChange={(e) => set(i, { organisation: e.target.value })} />
              <select className="input" aria-label="Attendance" value={r.attendance || "Present"} onChange={(e) => set(i, { attendance: e.target.value })}>
                <option>Present</option>
                <option>Online</option>
                <option>Absent</option>
              </select>
              <button type="button" className="btn btnDanger btnCompact" aria-label={`Remove ${r.name || "attendee"}`} onClick={() => setRows(rows.filter((_, j) => j !== i))}>
                Remove
              </button>
            </div>
          ))}
          <div>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setRows([...rows, blank()])}>
              Add attendee
            </button>
          </div>
        </div>

        {update.error && (
          <p className="formError" role="alert">
            {update.error.message}
          </p>
        )}
        <div className={styles.sheetFooter}>
          <label className={styles.remember}>
            <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
            Remember the ones I add or change
          </label>
          <span className={styles.grow} />
          <button type="button" className="btn btnGhost" onClick={close}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={update.isPending}
            onClick={() =>
              update.mutate(
                {
                  attendees: rows
                    .filter((r) => r.name.trim())
                    .map((r) => ({ name: r.name, title: r.title, organisation: r.organisation, attendance: r.attendance, heard_as: r.heardAs, changed: changed(r) })),
                  remember,
                },
                {
                  onSuccess: () => {
                    dialog.current?.close();
                    onSaved();
                  },
                },
              )
            }
          >
            {update.isPending ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
