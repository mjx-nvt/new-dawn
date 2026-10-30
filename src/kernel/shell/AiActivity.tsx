import { useEffect, useRef, useState } from "react";
import { useAiActivity, type Activity } from "../bridge/activity";
import { cancelTranscription, useTranscriptionProgress } from "../bridge/meetings";
import { cancelSummary, useSummaryProgress } from "../bridge/summary";
import { SummaryBar, TranscriptionBar } from "../../modules/meetings/AiProgress";
import { SparkleIcon } from "./icons";
import styles from "./AiActivity.module.css";

/** "Writing the summary of “Weekly sync”" → "Writing the summary": the toolbar has no room for titles. */
const verb = (label: string) => label.split(" “")[0].replace(/ of$/, "");

/** Toolbar: what the AI is doing now and what waits, while anything does (D-048). */
export default function AiActivity() {
  const { data: items } = useAiActivity();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const shown = items ?? [];

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => !root.current?.contains(event.target as Node) && setOpen(false);
    const onKeyDown = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);
  useEffect(() => {
    if (!shown.length) setOpen(false);
  }, [shown.length]);

  if (!shown.length) return null;
  const [first, ...rest] = shown;
  return (
    <div className={styles.root} ref={root}>
      <button
        type="button"
        className={`${styles.pill} aiMark`}
        aria-expanded={open}
        aria-haspopup="dialog"
        title="What the AI is doing"
        onClick={() => setOpen(!open)}
      >
        <SparkleIcon size={12} />
        <span className={styles.label}>{first.running ? verb(first.task.label) : "Waiting"}</span>
        {rest.length > 0 && <span className={styles.more}>+{rest.length}</span>}
      </button>
      {open && (
        <div className={`glass ${styles.popover}`} role="dialog" aria-label="AI activity">
          <p className={styles.heading}>AI on this Mac</p>
          <ul className={styles.list}>
            {shown.map((item, i) => (
              <Item key={`${item.task.label}-${i}`} item={item} />
            ))}
          </ul>
          <p className="fieldHint">One job at a time. What you're waiting for goes first; a long job pauses at its next step.</p>
        </div>
      )}
    </div>
  );
}

function Item({ item }: { item: Activity }) {
  const meetingId = item.task.meeting_id ?? "";
  const transcription = useTranscriptionProgress(meetingId);
  const summary = useSummaryProgress(meetingId);
  const kind = item.task.kind;
  const state = item.running ? null : item.lane === "interactive" ? "Next" : "Waiting";
  const stop = kind === "transcription" ? cancelTranscription : kind === "summary" ? cancelSummary : null;
  // A waiting job notices the cancel when its turn comes: say so until it leaves the list.
  const [stopping, setStopping] = useState(false);
  return (
    <li className={styles.item}>
      <div className={styles.itemHead}>
        <span className={styles.itemLabel}>{item.task.label}</span>
        {state && !stopping && <span className="chip">{state}</span>}
        {stop && (
          <button
            type="button"
            className="btn btnGhost btnCompact"
            disabled={stopping}
            onClick={() => {
              setStopping(true);
              void stop(meetingId);
            }}
          >
            {stopping ? "Stopping…" : "Cancel"}
          </button>
        )}
      </div>
      {item.running && kind === "transcription" && transcription && <TranscriptionBar progress={transcription} />}
      {item.running && kind === "summary" && summary && <SummaryBar progress={summary} />}
    </li>
  );
}
