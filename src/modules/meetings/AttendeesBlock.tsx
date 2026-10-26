import { useEffect, useRef, useState, type ComponentProps } from "react";
import { SparkleIcon } from "../../kernel/shell/icons";
import type { Attendee_Serialize } from "../../bindings";
import { useUpdateAttendees } from "../../kernel/bridge/people";
import { useSummarise } from "../../kernel/bridge/summary";
import { foldName } from "./format";
import styles from "./AttendeesBlock.module.css";

interface Row {
  name: string;
  title: string;
  organisation: string;
  attendance: string;
  /** In the attendees (the minutes). */
  attending: boolean;
  /** The spelling the row had when editing began (kept as "also heard as" if changed). */
  heardAs: string | null;
  /** The row as it was (null = added now): only added or changed rows teach the memory. */
  original: Attendee_Serialize | null;
}

const toRow = (a: Attendee_Serialize): Row => ({ ...a, heardAs: a.name || null, original: a });
const blank = (): Row => ({ name: "", title: "", organisation: "", attendance: "Present", attending: true, heardAs: null, original: null });

const changed = (r: Row) =>
  !r.original ||
  r.name !== r.original.name ||
  r.title !== r.original.title ||
  r.organisation !== r.original.organisation ||
  r.attendance !== r.original.attendance;

/** Everyone the meeting involves, in two lists: the attendees (the only people in the minutes)
 *  and the others. ⊖ / ⊕ move someone across. Edit opens a sheet; saving a name fixes it
 *  across the summary, email and minutes and (with Remember) teaches the people memory. */
export default function AttendeesBlock({
  slug,
  meetingId,
  attendees,
  you,
  locked,
}: {
  slug: string;
  meetingId: string;
  attendees: Attendee_Serialize[];
  /** Your name (from your profile), for the "You" chip. */
  you: string;
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
      return { name: r.name, title: r.title, organisation: r.organisation, attendance: r.attendance, attending: r.attending, heard_as: r.heardAs, changed: changed(r) };
    });
    update.mutate({ attendees: list.filter((a) => a.name.trim()), remember: true }, { onSuccess: () => { setInline(null); setSaved(true); } });
  };

  // ⊖ / ⊕ / Add all (`i` = -1): the same list in the same order, only the flags change —
  // nothing is renamed or remembered. Disabled while saving, so two quick clicks can't undo each
  // other. The person lands in the other list (a new button) and Add all goes away, so focus
  // follows them — or goes to the Attendees heading — once their button is enabled again.
  const moved = useRef<number | null>(null);
  const setAttending = (i: number, attending: boolean) => {
    moved.current = i;
    update.mutate({
      attendees: attendees
        .map((a, j) => ({ name: a.name, title: a.title, organisation: a.organisation, attendance: a.attendance, attending: i === -1 || i === j ? attending : a.attending, heard_as: null, changed: false }))
        .filter((a) => a.name.trim()),
      remember: false,
    });
  };
  const refocus = (i: number) => (el: HTMLElement | null) => {
    if (el && moved.current === i && !el.matches(":disabled")) {
      moved.current = null;
      el.focus();
    }
  };

  const me = foldName(you);
  const people = attendees.map((a, i) => ({ a, i }));
  const going = people.filter(({ a }) => a.attending);
  const others = people.filter(({ a }) => !a.attending);
  const busy = locked || update.isPending;

  // A person: the ⊖ / ⊕ control, then the name (click to edit) and its chips over the role.
  const item = ({ a, i }: { a: Attendee_Serialize; i: number }, control: React.ReactNode) => {
    const role = [a.title, a.organisation].filter(Boolean).join(", ");
    return inline === i ? (
      <li key={i} className={styles.inline}>
        <InlineEditor row={toRow(a)} saving={update.isPending} onCancel={() => setInline(null)} onSave={(row) => saveInline(i, row)} />
      </li>
    ) : (
      <li key={i}>
        {control}
        <div className={styles.who}>
          <p className={styles.line}>
            <button type="button" className={styles.name} disabled={locked} title="Edit in place" onClick={() => setInline(i)}>
              {a.name}
            </button>
            {me && foldName(a.name) === me && <span className="chip">You</span>}
            {a.attendance && a.attendance !== "Present" && <span className="chip">{a.attendance}</span>}
          </p>
          {role && <p className={styles.role}>{role}</p>}
        </div>
      </li>
    );
  };

  return (
    <div className={styles.block}>
      <div className={styles.head}>
        <h3 className="sectionTitle">All people</h3>
        <button type="button" className="btn btnGhost btnCompact" disabled={locked} onClick={() => setEditing(true)}>
          Edit
        </button>
      </div>

      <div className={styles.sub}>
        <div className={styles.subHead}>
          <h4 className="eyebrow" tabIndex={-1} ref={refocus(-1)}>Attendees</h4>
          <span className="fieldHint">In the minutes</span>
        </div>
        {going.length ? (
          <ul className={styles.list}>
            {going.map((p) =>
              item(p, <ListControl ref={refocus(p.i)} disabled={busy} label={`Remove ${p.a.name} from the attendees`} onClick={() => setAttending(p.i, false)} />),
            )}
          </ul>
        ) : (
          <>
            <p className="ghostText">Not yet captured</p>
            <div className={`${styles.slot} ghostCard`} />
            <div className={`${styles.slot} ghostCard`} />
          </>
        )}
      </div>

      <div className={styles.sub}>
        <div className={styles.subHead}>
          <h4 className="eyebrow">Also involved</h4>
          <span className="fieldHint">Not in the minutes</span>
          {others.length > 0 && (
            <button type="button" className={`btn btnGhost btnCompact ${styles.action}`} disabled={busy} onClick={() => setAttending(-1, true)}>
              Add all
            </button>
          )}
        </div>
        {others.length ? (
          <ul className={styles.list}>
            {others.map((p) =>
              item(p, <ListControl add ref={refocus(p.i)} disabled={busy} label={`Add ${p.a.name} to the attendees`} onClick={() => setAttending(p.i, true)} />),
            )}
          </ul>
        ) : attendees.length ? (
          <p className="fieldHint">Everyone the meeting involves is an attendee.</p>
        ) : (
          <p className="ghostText">Not yet captured</p>
        )}
      </div>

      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
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
          onSaved={(renamed) => {
            setEditing(false);
            if (renamed) setSaved(true);
          }}
        />
      )}
    </div>
  );
}

/** Apple's list-editing control at the start of a row: a red ⊖ takes the person out of a list, a
 *  green ⊕ puts them in (the one red that isn't record or destructive: it only moves people, so
 *  one click and no confirm). `label` names the person, for the tooltip and screen readers. */
function ListControl({ add = false, label, ...button }: { add?: boolean; label: string } & ComponentProps<"button">) {
  return (
    <button type="button" className={add ? `${styles.control} ${styles.add}` : styles.control} aria-label={label} title={label} {...button}>
      <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden>
        <circle cx="9" cy="9" r="9" />
        <path d={add ? "M5 9h8M9 5v8" : "M5 9h8"} />
      </svg>
    </button>
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
  /** `renamed`: a name or role was added or changed (not just ticks or removals). */
  onSaved: (renamed: boolean) => void;
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
            All people
          </h2>
          <p className="fieldHint">Ticked people are the attendees, the only ones in the minutes. Fix a name once: it's corrected everywhere and remembered.</p>
        </div>

        <div className={styles.table}>
          <div className={styles.tableHead} aria-hidden>
            <span className="fieldLabel">Attendee</span>
            <span className="fieldLabel">Name</span>
            <span className="fieldLabel">Job title</span>
            <span className="fieldLabel">Organisation</span>
            <span className="fieldLabel">Attendance</span>
            <span />
          </div>
          {rows.map((r, i) => (
            <div key={i} className={styles.tableRow}>
              <input type="checkbox" className={styles.tick} aria-label="Attendee" title="Attendee: in the minutes" checked={r.attending} onChange={(e) => set(i, { attending: e.target.checked })} />
              <input className="input" aria-label="Name" placeholder="Name" value={r.name} autoFocus={i === rows.length - 1 && !r.original} onChange={(e) => set(i, { name: e.target.value })} />
              <input className="input" aria-label="Job title" placeholder="Job title" value={r.title} onChange={(e) => set(i, { title: e.target.value })} />
              <input className="input" aria-label="Organisation" placeholder="Organisation" value={r.organisation} onChange={(e) => set(i, { organisation: e.target.value })} />
              <select className="input" aria-label="Attendance" value={r.attendance || "Present"} onChange={(e) => set(i, { attendance: e.target.value })}>
                <option>Present</option>
                <option>Online</option>
                <option>Absent</option>
              </select>
              <ListControl label={`Remove ${r.name || "this person"}`} onClick={() => setRows(rows.filter((_, j) => j !== i))} />
            </div>
          ))}
          <div>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setRows([...rows, blank()])}>
              Add person
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
                    .map((r) => ({ name: r.name, title: r.title, organisation: r.organisation, attendance: r.attendance, attending: r.attending, heard_as: r.heardAs, changed: changed(r) })),
                  remember,
                },
                {
                  onSuccess: () => {
                    dialog.current?.close();
                    onSaved(rows.some((r) => r.name.trim() && changed(r)));
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
