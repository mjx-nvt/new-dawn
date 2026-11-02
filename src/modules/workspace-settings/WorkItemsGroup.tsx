import { useEffect, useState } from "react";
import { usePmPrefs, useSetPmPrefs, type NoticeKind } from "../../kernel/bridge/notices";
import OnOff from "./OnOff";
import styles from "./Settings.module.css";

/** Plane's notification kinds, as the Inbox shows them (D-075). */
const KINDS: { kind: NoticeKind; label: string; hint: string }[] = [
  { kind: "property", label: "Property changes", hint: "Assignees, labels, priority, dates, links, relations, attachments — what an agent or an automation changes on a work item you follow." },
  { kind: "state", label: "State changes", hint: "A work item you follow moving to another state." },
  { kind: "completed", label: "Work item completed", hint: "One of them reaching a completed state." },
  { kind: "comment", label: "Comments", hint: "An agent's comment on a work item you follow." },
  { kind: "mention", label: "Mentions", hint: "A comment naming you." },
  { kind: "reminder", label: "Due-date reminders", hint: "The day before a work item you follow is due (projects with reminders on)." },
];

/** Settings → General → Inbox and Work items: which notices the Inbox shows (in the app only —
 *  nothing is mailed), and how large an attachment may be. */
export default function WorkItemsGroup() {
  const { data: prefs } = usePmPrefs();
  const save = useSetPmPrefs();
  const [mb, setMb] = useState("");
  useEffect(() => setMb(prefs ? String(prefs.attachment_mb) : ""), [prefs]);
  const muted = prefs?.muted ?? [];
  const keepMb = () => {
    const n = Math.round(Number(mb));
    if (prefs && n !== prefs.attachment_mb) save.mutate({ ...prefs, attachment_mb: n }, { onError: () => setMb(String(prefs.attachment_mb)) });
  };
  return (
    <>
      <p className="groupTitle">Inbox</p>
      <div className="group">
        {KINDS.map((k) => (
          <div key={k.kind} className="groupRow">
            <div className="rowText">
              <p className="rowLabel">{k.label}</p>
              <p className="rowHint">{k.hint}</p>
            </div>
            <div className="rowControl">
              <OnOff label={k.label} value={prefs ? !muted.includes(k.kind) : undefined} disabled={!prefs || save.isPending} onChange={(on) => prefs && save.mutate({ ...prefs, muted: on ? muted.filter((m) => m !== k.kind) : [...muted, k.kind] })} />
            </div>
          </div>
        ))}
      </div>
      <p className="groupTitle">Work items</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Attachment size limit</p>
            <p className="rowHint">The largest file a work item takes, in MB (1 to 200). Files are copied into the workspace's folder.</p>
          </div>
          <div className="rowControl">
            <input
              className={`input ${styles.numInput}`}
              type="number"
              min={1}
              max={200}
              aria-label="Attachment size limit in MB"
              value={mb}
              disabled={!prefs}
              onChange={(e) => setMb(e.target.value)}
              onBlur={keepMb}
              onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
            />
          </div>
        </div>
      </div>
      {save.error && <p className="formError">{save.error.message}</p>}
    </>
  );
}
