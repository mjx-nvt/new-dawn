import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import {
  cancelTranscription,
  useDeleteMeeting,
  useMeeting,
  useRenameMeeting,
  useRetranscribe,
  useTranscriptionError,
  useTranscriptionProgress,
} from "../../kernel/bridge/meetings";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatClock, formatWhen, LANGUAGE_LABEL } from "./format";
import styles from "./MeetingPage.module.css";
import MinutesTab from "./MinutesTab";
import ProgressBar from "./ProgressBar";
import StatusChip from "./StatusChip";
import SummaryTab from "./SummaryTab";
import TranscriptView from "./TranscriptView";

const route = getRouteApi("/shell/meetings/$meetingId");

export const MEETING_TABS = [
  { id: "transcript", label: "Transcript" },
  { id: "summary", label: "Summary" },
  { id: "minutes", label: "Minutes" },
] as const;

export type MeetingTab = (typeof MEETING_TABS)[number]["id"];

/** One meeting: header, then Transcript | Summary | Minutes (tab in the URL). */
export default function MeetingPage() {
  const { meetingId } = route.useParams();
  const { tab, t } = route.useSearch();
  const { data: workspace } = useCurrentWorkspace();
  const meeting = useMeeting(workspace?.slug, meetingId);
  const progress = useTranscriptionProgress(meetingId);
  const lastError = useTranscriptionError(meetingId);
  const retranscribe = useRetranscribe();
  const [renaming, setRenaming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmRetranscribe, setConfirmRetranscribe] = useState(false); // replaces corrections too

  useEffect(() => {
    if (!confirmRetranscribe) return;
    const t = setTimeout(() => setConfirmRetranscribe(false), 4000);
    return () => clearTimeout(t);
  }, [confirmRetranscribe]);

  if (meeting.error) {
    return (
      <div className="page">
        <p className="eyebrow">{workspace?.name}</p>
        <h1 className="pageTitle">Meeting not found</h1>
        <p className="pageMeta">It may belong to another workspace.</p>
        <Link to="/meetings" search={{}} className={`btn btnSecondary ${styles.back}`}>
          Back to meetings
        </Link>
      </div>
    );
  }

  const m = meeting.data;
  const canRetranscribe = m && (m.status === "recorded" || m.status === "failed" || m.status === "transcribed");
  const busy = !m || m.status === "recording" || m.status === "transcribing" || m.status === "summarising";

  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name}</p>
      <div className={styles.header}>
        {renaming && m && workspace ? (
          <TitleEditor slug={workspace.slug} meetingId={meetingId} title={m.title} onDone={() => setRenaming(false)} />
        ) : (
          <h1 className="pageTitle">{m?.title ?? ""}</h1>
        )}
        <div className={styles.actions}>
          {!renaming && (
            <button type="button" className="btn btnGhost btnCompact" disabled={!m} onClick={() => setRenaming(true)}>
              Rename
            </button>
          )}
          <button
            type="button"
            className="btn btnGhost btnCompact"
            disabled={busy}
            title={busy ? "Wait until recording, transcription or the summary has finished" : undefined}
            onClick={() => setDeleting(true)}
          >
            Delete…
          </button>
          {canRetranscribe && workspace && (
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={retranscribe.isPending}
              title={m.status === "transcribed" ? "Replaces the transcript, including your corrections" : undefined}
              onClick={() => {
                if (m.status === "transcribed" && !confirmRetranscribe) return setConfirmRetranscribe(true);
                setConfirmRetranscribe(false);
                retranscribe.mutate({ slug: workspace.slug, meetingId });
              }}
            >
              {m.status !== "transcribed" ? "Transcribe" : confirmRetranscribe ? "Replace transcript and corrections?" : "Transcribe again"}
            </button>
          )}
        </div>
      </div>
      {m && (
        <p className={`pageMeta ${styles.meta}`}>
          {formatWhen(m.held_on)}
          {m.duration_ms != null && <> · {formatClock(m.duration_ms)}</>} · {LANGUAGE_LABEL[m.language]}
          <StatusChip status={m.status} />
        </p>
      )}
      {(progress || m?.status === "transcribing") && (
        <div className={`card ${styles.progressCard}`}>
          <div className={styles.progressHead}>
            <p className={styles.progressTitle}>Transcribing{progress ? ` · ${progress.percent} %` : "…"}</p>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => void cancelTranscription(meetingId)}>
              Cancel
            </button>
          </div>
          {progress ? <ProgressBar progress={progress} label={m?.title} /> : <p className="fieldHint">Waiting for the job to report progress</p>}
          <p className="fieldHint">You can leave this page — you'll get a notification when the transcript is ready.</p>
        </div>
      )}
      {(lastError ?? retranscribe.error) && <p className={styles.error}>{(lastError ?? retranscribe.error)?.message}</p>}

      <nav className={styles.tabs}>
        {MEETING_TABS.map((item) => (
          <Link
            key={item.id}
            to="/meetings/$meetingId"
            params={{ meetingId }}
            search={{ tab: item.id }}
            replace
            className={item.id === tab ? `${styles.tab} ${styles.activeTab}` : styles.tab}
          >
            {item.label}
          </Link>
        ))}
      </nav>

      {tab === "transcript" && m && workspace && <TranscriptView key={m.id} slug={workspace.slug} meeting={m} startMs={t} />}
      {tab === "summary" && m && workspace && <SummaryTab slug={workspace.slug} meeting={m} />}
      {tab === "minutes" && m && workspace && <MinutesTab slug={workspace.slug} meeting={m} />}
      {deleting && m && workspace && <DeleteSheet slug={workspace.slug} meetingId={meetingId} title={m.title} onClose={() => setDeleting(false)} />}
    </div>
  );
}

/** Inline title field: Enter saves, Esc cancels. */
function TitleEditor({ slug, meetingId, title, onDone }: { slug: string; meetingId: string; title: string; onDone: () => void }) {
  const [draft, setDraft] = useState(title);
  const rename = useRenameMeeting(slug, meetingId);
  const save = () => {
    if (draft.trim() === title) return onDone();
    rename.mutate(draft, { onSuccess: onDone });
  };
  return (
    <div className={styles.titleEditor}>
      <input
        className={`input ${styles.titleInput}`}
        autoFocus
        value={draft}
        maxLength={200}
        aria-label="Meeting title"
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.nativeEvent.isComposing) save();
          if (e.key === "Escape") onDone();
        }}
      />
      <button type="button" className="btn btnPrimary btnCompact" disabled={!draft.trim() || rename.isPending} onClick={save}>
        Save
      </button>
      <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>
        Cancel
      </button>
      {rename.error && (
        <p className={styles.error} role="alert">
          {rename.error.message}
        </p>
      )}
    </div>
  );
}

/** Confirm sheet: the recording goes to the Trash; exported minutes stay in the exports folder. */
function DeleteSheet({ slug, meetingId, title, onClose }: { slug: string; meetingId: string; title: string; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const remove = useDeleteMeeting(slug, meetingId);
  const navigate = useNavigate();
  const close = () => !remove.isPending && onClose();

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="delete-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="delete-title" className="sheetHeading">
          Delete “{title}”?
        </h2>
        <p className="fieldHint">
          The recording moves to the Trash (you can put it back from there). The transcript, summary, notes and export list are
          deleted. Minutes you already exported stay in the workspace's exports folder.
        </p>
        {remove.error && (
          <p className="formError" role="alert">
            {remove.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" autoFocus onClick={close}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={remove.isPending}
            onClick={() => remove.mutate(undefined, { onSuccess: () => void navigate({ to: "/meetings", search: {} }) })}
          >
            Delete meeting
          </button>
        </div>
      </div>
    </dialog>
  );
}
