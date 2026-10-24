import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useMeetings, useSearchMeetings, useTranscriptionProgress, type Meeting } from "../../kernel/bridge/meetings";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { formatClock, formatWhen } from "./format";
import ImportAudioSheet from "./ImportAudioSheet";
import styles from "./MeetingsPage.module.css";
import ProgressBar from "./ProgressBar";
import Snippet from "./Snippet";
import StatusChip from "./StatusChip";

const route = getRouteApi("/shell/meetings");

/** The current workspace's meetings, newest first, with full-text search (?q=); ?import=true opens
 *  the import sheet. */
export default function MeetingsPage() {
  const { data: workspace } = useCurrentWorkspace();
  const { import: importOpen, q = "" } = route.useSearch();
  const meetings = useMeetings(workspace?.slug);
  const search = useSearchMeetings(workspace?.slug, q);
  const navigate = useNavigate();
  const setSearch = (next: { import?: true; q?: string }, replace = true) => void navigate({ to: "/meetings", search: next, replace });
  const searching = q.trim().length > 0;
  const error = searching ? search.error : meetings.error;

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">Meetings</h1>
        </div>
        <button type="button" className="btn btnPrimary" disabled={!workspace} onClick={() => setSearch({ q: q || undefined, import: true }, false)}>
          Import audio…
        </button>
      </div>

      <search>
        <input
          className={`input ${styles.search}`}
          type="search"
          placeholder="Search titles, transcripts and summaries"
          aria-label="Search meetings"
          value={q}
          onChange={(e) => setSearch(e.target.value ? { q: e.target.value } : {})}
        />
      </search>

      <section className="section">
        <h2 className="sectionTitle">{searching ? "Results" : "Recent meetings"}</h2>
        <p className="sectionSubtitle" aria-live="polite">
          {searching
            ? `${search.data?.length ?? 0} meeting${search.data?.length === 1 ? "" : "s"} mention “${q.trim()}”`
            : "Recorded calls with their transcript, summary and minutes"}
        </p>
        {error && <p className={styles.error}>{error.message}</p>}
        <div className={styles.list}>
          {searching
            ? search.data?.length
              ? search.data.map((hit) => <MeetingRow key={hit.meeting.id} meeting={hit.meeting} snippet={hit.snippet} atMs={hit.at_ms} />)
              : [
                  <p key="none" className="ghostText">
                    {search.isPending ? "Searching…" : "No meetings match"}
                  </p>,
                  ...[0, 1].map((i) => <div key={i} className={`${styles.slot} ghostCard`} />),
                ]
            : meetings.data?.length
              ? meetings.data.map((meeting) => <MeetingRow key={meeting.id} meeting={meeting} />)
              : [0, 1, 2].map((i) => <div key={i} className={`${styles.slot} ghostCard`} />)}
        </div>
      </section>

      {importOpen && <ImportAudioSheet onClose={() => setSearch(q ? { q } : {})} />}
    </div>
  );
}

interface RowProps {
  meeting: Meeting;
  snippet?: string | null;
  atMs?: number | null;
}

function MeetingRow({ meeting, snippet, atMs }: RowProps) {
  const progress = useTranscriptionProgress(meeting.id);
  return (
    <Link
      to="/meetings/$meetingId"
      params={{ meetingId: meeting.id }}
      search={atMs != null ? { tab: "transcript", t: atMs } : { tab: snippet ? "summary" : "transcript" }}
      className={`card ${styles.row}`}
    >
      <div className={styles.main}>
        <p className={styles.title}>{meeting.title}</p>
        <p className="fieldHint">
          {formatWhen(meeting.held_on)}
          {meeting.duration_ms != null && ` · ${formatClock(meeting.duration_ms)}`}
          {atMs != null && ` · match at ${formatClock(atMs)}`}
        </p>
        {snippet && <Snippet text={snippet} />}
        {progress && <ProgressBar progress={progress} label={meeting.title} />}
      </div>
      <StatusChip status={meeting.status} />
    </Link>
  );
}
