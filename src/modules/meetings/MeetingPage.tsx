import { getRouteApi, Link } from "@tanstack/react-router";
import {
  cancelTranscription,
  useMeeting,
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

  return (
    <div className="page">
      <p className="eyebrow">{workspace?.name}</p>
      <div className={styles.header}>
        <h1 className="pageTitle">{m?.title ?? ""}</h1>
        <div className={styles.actions}>
          {canRetranscribe && workspace && (
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={retranscribe.isPending}
              onClick={() => retranscribe.mutate({ slug: workspace.slug, meetingId })}
            >
              {m.status === "transcribed" ? "Transcribe again" : "Transcribe"}
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

      {tab === "transcript" && m && workspace && <TranscriptView slug={workspace.slug} meeting={m} startMs={t} />}
      {tab === "summary" && m && workspace && <SummaryTab slug={workspace.slug} meeting={m} />}
      {tab === "minutes" && m && workspace && <MinutesTab slug={workspace.slug} meeting={m} />}
    </div>
  );
}
