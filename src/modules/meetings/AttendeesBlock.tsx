import { useState } from "react";
import type { Attendee_Serialize } from "../../bindings";
import { useUpdateAttendees } from "../../kernel/bridge/people";
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

const changed = (r: Row) =>
  !r.original ||
  r.name !== r.original.name ||
  r.title !== r.original.title ||
  r.organisation !== r.original.organisation ||
  r.attendance !== r.original.attendance;

/** Who attended: corrected names, titles and organisations go into the minutes and the email,
 *  and (with Remember) into the workspace's people memory for the next meeting. */
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
  const update = useUpdateAttendees(slug, meetingId);
  const [rows, setRows] = useState<Row[] | null>(null);
  const [remember, setRemember] = useState(true);
  const set = (i: number, patch: Partial<Row>) => setRows((rs) => rs && rs.map((r, j) => (j === i ? { ...r, ...patch } : r)));

  return (
    <div className={styles.block}>
      <div className={styles.head}>
        <h3 className="sectionTitle">Attendees</h3>
        {rows === null && (
          <button
            type="button"
            className="btn btnGhost btnCompact"
            disabled={locked}
            onClick={() => {
              update.reset();
              setRows(attendees.map(toRow));
            }}
          >
            Edit
          </button>
        )}
      </div>

      {rows === null ? (
        attendees.length ? (
          <ul className={styles.list}>
            {attendees.map((a, i) => (
              <li key={i}>
                <span className={styles.name}>{a.name}</span>
                {[a.title, a.organisation].filter(Boolean).join(", ") && <span> — {[a.title, a.organisation].filter(Boolean).join(", ")}</span>}
                {a.attendance && a.attendance !== "Present" && <span className="chip">{a.attendance}</span>}
              </li>
            ))}
          </ul>
        ) : (
          <>
            <p className="ghostText">Not yet captured</p>
            <div className={`${styles.slot} ghostCard`} />
            <div className={`${styles.slot} ghostCard`} />
          </>
        )
      ) : (
        <div className={styles.editor}>
          <div className={styles.grid}>
            <div className={styles.headings} aria-hidden>
              <span className="fieldLabel">Name</span>
              <span className="fieldLabel">Job title</span>
              <span className="fieldLabel">Organisation</span>
              <span className="fieldLabel">Attendance</span>
              <span />
            </div>
            {rows.map((r, i) => (
              <Row key={i} row={r} onChange={(patch) => set(i, patch)} onRemove={() => setRows(rows.filter((_, j) => j !== i))} />
            ))}
          </div>
          <div className={styles.actions}>
            <button
              type="button"
              className="btn btnGhost btnCompact"
              onClick={() =>
                setRows([...rows, { name: "", title: "", organisation: "", attendance: "Present", heardAs: null, original: null }])
              }
            >
              Add attendee
            </button>
            <label className={styles.remember}>
              <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
              Remember the ones I add or change
            </label>
            <span className={styles.grow} />
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setRows(null)}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btnPrimary btnCompact"
              disabled={locked || update.isPending}
              onClick={() =>
                update.mutate(
                  {
                    attendees: rows.map((r) => ({
                      name: r.name,
                      title: r.title,
                      organisation: r.organisation,
                      attendance: r.attendance,
                      heard_as: r.heardAs,
                      changed: changed(r),
                    })),
                    remember,
                  },
                  { onSuccess: () => setRows(null) },
                )
              }
            >
              Save
            </button>
          </div>
          {update.error && (
            <p className={styles.error} role="alert">
              {update.error.message}
            </p>
          )}
        </div>
      )}
    </div>
  );
}

function Row({ row, onChange, onRemove }: { row: Row; onChange: (patch: Partial<Row>) => void; onRemove: () => void }) {
  return (
    <div className={styles.person}>
      <input className="input" aria-label="Name" placeholder="Name" value={row.name} onChange={(e) => onChange({ name: e.target.value })} />
      <input className="input" aria-label="Job title" placeholder="Job title" value={row.title} onChange={(e) => onChange({ title: e.target.value })} />
      <input
        className="input"
        aria-label="Organisation"
        placeholder="Organisation"
        value={row.organisation}
        onChange={(e) => onChange({ organisation: e.target.value })}
      />
      <select className="input" aria-label="Attendance" value={row.attendance || "Present"} onChange={(e) => onChange({ attendance: e.target.value })}>
        <option>Present</option>
        <option>Online</option>
        <option>Absent</option>
      </select>
      <button type="button" className="btn btnGhost btnCompact" aria-label={`Remove ${row.name || "attendee"}`} onClick={onRemove}>
        Remove
      </button>
    </div>
  );
}
