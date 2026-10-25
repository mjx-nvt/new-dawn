import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import {
  cancelTranscription,
  useDeleteMeeting,
  useMeeting,
  useMeetingTypes,
  useRenameMeeting,
  useRetranscribe,
  useSetMeetingType,
  useTranscriptionError,
  useTranscriptionProgress,
  type Meeting,
  useSetMeetingFormat,
} from "../../kernel/bridge/meetings";
import { useOllamaStatus, useSummarise } from "../../kernel/bridge/summary";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { SparkleIcon } from "../../kernel/shell/icons";
import MenuButton from "../../kernel/shell/MenuButton";
import { formatClock, formatWhen, LANGUAGE_LABEL } from "./format";
import styles from "./MeetingPage.module.css";
import MinutesTab from "./MinutesTab";
import { TranscriptionBar } from "./AiProgress";
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

/** One meeting: title and facts, the one action for its current step, then
 *  Transcript | Summary | Minutes (the tab lives in the URL). */
export default function MeetingPage() {
  const { meetingId } = route.useParams();
  const { tab, t } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug ?? "";
  const meeting = useMeeting(workspace?.slug, meetingId);
  const progress = useTranscriptionProgress(meetingId);
  const lastError = useTranscriptionError(meetingId);
  const retranscribe = useRetranscribe();
  const summarise = useSummarise();
  const ollama = useOllamaStatus();
  const [renaming, setRenaming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmRetranscribe, setConfirmRetranscribe] = useState(false);

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
  const busy = !m || m.status === "recording" || m.status === "transcribing" || m.status === "summarising";
  const ollamaReady = !!ollama.data?.running && !!ollama.data.model_installed;
  const showTab = (next: MeetingTab) => void navigate({ to: "/meetings/$meetingId", params: { meetingId }, search: { tab: next }, replace: true });
  const error = lastError ?? retranscribe.error ?? summarise.error;

  // The one thing to do next, by where the meeting is.
  const primary = (() => {
    if (!m || !workspace) return null;
    switch (m.status) {
      case "recorded":
      case "failed":
        return { label: "Transcribe", ai: true, busy: retranscribe.isPending, onClick: () => retranscribe.mutate({ slug, meetingId }), disabled: retranscribe.isPending };
      case "transcribed":
        return {
          label: "Write summary",
          ai: true,
          busy: summarise.isPending,
          onClick: () => {
            summarise.mutate({ slug, meetingId, deep: false, keepEdits: true });
            showTab("summary");
          },
          disabled: !ollamaReady || summarise.isPending,
          title: ollamaReady ? undefined : "Ollama isn't running with the text model — see Settings → Models",
        };
      case "summarised":
        return { label: "Export minutes", ai: false, busy: false, onClick: () => showTab("minutes"), disabled: false };
      default:
        return null;
    }
  })();

  return (
    <div className="page">
      <header className={styles.header}>
        <div className={styles.heading}>
          <p className="eyebrow">{workspace?.name}</p>
          {renaming && m && workspace ? (
            <TitleEditor slug={slug} meetingId={meetingId} title={m.title} onDone={() => setRenaming(false)} />
          ) : (
            <h1 className="pageTitle" title="Double-click to rename" onDoubleClick={() => m && setRenaming(true)}>
              {m?.title ?? ""}
            </h1>
          )}
          {m && (
            <div className={styles.facts}>
              <span className={styles.fact}>{formatWhen(m.held_on)}</span>
              {m.duration_ms != null && <span className={styles.fact}>{formatClock(m.duration_ms)}</span>}
              <span className={styles.fact}>{LANGUAGE_LABEL[m.language]}</span>
              <StatusChip status={m.status} />
              {workspace && <TypePicker slug={slug} meeting={m} />}
              {workspace && <FormatPicker slug={slug} meeting={m} />}
            </div>
          )}
        </div>
        {m && workspace && (
          <div className={styles.actions}>
            {primary && (
              <button
                type="button"
                className={primary.ai ? "btn btnPrimary btnAi" : "btn btnPrimary"}
                aria-busy={primary.busy || undefined}
                disabled={primary.disabled}
                title={primary.title}
                onClick={primary.onClick}
              >
                {primary.ai && <SparkleIcon />}
                {primary.label}
              </button>
            )}
            <MenuButton
              className="btn btnGhost"
              items={[
                { label: "Rename…", onSelect: () => setRenaming(true) },
                {
                  label: m.status === "transcribed" || m.status === "summarised" ? "Transcribe again…" : "Transcribe",
                  onSelect: () => (m.status === "recorded" || m.status === "failed" ? retranscribe.mutate({ slug, meetingId }) : setConfirmRetranscribe(true)),
                  disabled: busy || retranscribe.isPending,
                },
                { label: "Delete…", onSelect: () => setDeleting(true), danger: true, disabled: busy, separator: true },
              ]}
            />
          </div>
        )}
      </header>

      {confirmRetranscribe && workspace && (
        <div className={`card ${styles.notice}`} role="alertdialog" aria-labelledby="retranscribe-title">
          <div>
            <p id="retranscribe-title" className={styles.noticeTitle}>
              Transcribe again?
            </p>
            <p className="fieldHint">Lines you corrected are replaced. The summary stays until you re-run it.</p>
          </div>
          <div className={styles.noticeActions}>
            <button type="button" className="btn btnGhost btnCompact" autoFocus onClick={() => setConfirmRetranscribe(false)}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btnPrimary btnCompact btnAi"
              onClick={() => {
                setConfirmRetranscribe(false);
                retranscribe.mutate({ slug, meetingId });
              }}
            >
              <SparkleIcon />
              Transcribe again
            </button>
          </div>
        </div>
      )}

      {(progress || m?.status === "transcribing") && (
        <div className={`card ${styles.progressCard}`}>
          <div className={styles.progressHead}>
            <p className={styles.noticeTitle}>Transcribing{progress ? ` · ${progress.percent} %` : "…"}</p>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => void cancelTranscription(meetingId)}>
              Cancel
            </button>
          </div>
          {progress ? <TranscriptionBar progress={progress} title={m?.title} /> : <p className="fieldHint">Waiting for the job to report progress</p>}
          <p className="fieldHint">You'll be notified when the transcript is ready.</p>
        </div>
      )}
      {error && (
        <p className={styles.error} role="alert">
          {error.message}
        </p>
      )}

      <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="Meeting">
        {MEETING_TABS.map((item) => (
          <button key={item.id} type="button" role="tab" aria-selected={item.id === tab} onClick={() => showTab(item.id)}>
            {item.label}
          </button>
        ))}
      </div>

      <div role="tabpanel">
        {tab === "transcript" && m && workspace && <TranscriptView key={m.id} slug={slug} meeting={m} startMs={t} />}
        {tab === "summary" && m && workspace && <SummaryTab slug={slug} meeting={m} />}
        {tab === "minutes" && m && workspace && <MinutesTab slug={slug} meeting={m} />}
      </div>
      {deleting && m && workspace && <DeleteSheet slug={slug} meetingId={meetingId} title={m.title} onClose={() => setDeleting(false)} />}
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

/** Discussion, or one presenter (named when the recording may not say it) — read by the next
 *  transcription (Whisper hears the name) and summary (who said what), D-038. */
function FormatPicker({ slug, meeting }: { slug: string; meeting: Meeting }) {
  const set = useSetMeetingFormat(slug, meeting.id);
  const presentation = meeting.format === "presentation";
  const [name, setName] = useState(meeting.presenter ?? "");
  useEffect(() => setName(meeting.presenter ?? ""), [meeting.presenter]);
  const save = (p: boolean, presenter: string) => set.mutate({ presentation: p, presenter: presenter.trim() || null });
  return (
    <>
      <select className="input" aria-label="Meeting format" value={presentation ? "presentation" : "discussion"} disabled={set.isPending} onChange={(e) => save(e.target.value === "presentation", name)}>
        <option value="discussion">Discussion</option>
        <option value="presentation">One presenter</option>
      </select>
      {presentation && (
        <input
          className={`input ${styles.presenter}`}
          aria-label="Presenter"
          placeholder="Presenter's name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onBlur={() => name.trim() !== (meeting.presenter ?? "") && save(true, name)}
          onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
        />
      )}
      {set.error && (
        <span className={styles.error} role="alert">
          {set.error.message}
        </span>
      )}
    </>
  );
}

/** The meeting's type: picks which Word template its minutes use. */
function TypePicker({ slug, meeting }: { slug: string; meeting: Meeting }) {
  const types = useMeetingTypes(slug);
  const setType = useSetMeetingType(slug, meeting.id);
  if (!types.data?.length) return null;
  return (
    <>
      <select
        className="input"
        aria-label="Meeting type"
        value={meeting.meeting_type_id ?? ""}
        disabled={setType.isPending}
        onChange={(e) => setType.mutate(e.target.value || null)}
      >
        <option value="">No type</option>
        {types.data.map((type) => (
          <option key={type.id} value={type.id}>
            {type.name}
          </option>
        ))}
      </select>
      {setType.error && (
        <span className={styles.error} role="alert">
          {setType.error.message}
        </span>
      )}
    </>
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
          The recording moves to the Trash. The transcript, summary, notes and export list are deleted. Minutes you already
          exported stay in the workspace's exports folder.
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
            className="btn btnDangerFilled"
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
