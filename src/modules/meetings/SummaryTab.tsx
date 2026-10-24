import { useEffect, useState } from "react";
import type { Meeting } from "../../kernel/bridge/meetings";
import {
  cancelSummary,
  openOllamaApp,
  useCopyEmail,
  useOllamaStatus,
  useOpenInMail,
  useSummarise,
  useSummary,
  useSummaryError,
  useSummaryProgress,
  useUpdateSection,
  type SummaryProgress,
  type SummaryView,
} from "../../kernel/bridge/summary";
import { formatWhen } from "./format";
import styles from "./SummaryTab.module.css";

// Until meeting types drive the structure (P1.4), the same sections the backend uses by default.
const SKELETON = ["Overview", "Key points", "Decisions", "Action items", "Next steps", "Open questions"];

/** Executive summary: written by Qwen after the transcript, editable, copied as an email. */
export default function SummaryTab({ slug, meeting }: { slug: string; meeting: Meeting }) {
  const summary = useSummary(slug, meeting.id);
  const progress = useSummaryProgress(meeting.id);
  const lastError = useSummaryError(meeting.id);
  const ollama = useOllamaStatus();
  const summarise = useSummarise();
  const writing = !!progress || meeting.status === "summarising";
  const transcribed = ["transcribed", "summarising", "summarised"].includes(meeting.status);
  const ollamaReady = !!ollama.data?.running && !!ollama.data.model_installed;
  // The newest problem wins: a fresh start attempt clears an older job error and vice versa.
  const error = summarise.error ?? lastError ?? summary.error;
  const run = (deep: boolean) => summarise.mutate({ slug, meetingId: meeting.id, deep });

  return (
    <section className="section">
      {ollama.data && !ollamaReady && !writing && <OllamaCard running={ollama.data.running} model={ollama.data.model} />}
      {writing && <ProgressCard meetingId={meeting.id} progress={progress} />}
      {error && (
        <p className={styles.error} role="alert">
          {error.message}
        </p>
      )}

      {summary.data ? (
        <SummaryBody slug={slug} meeting={meeting} view={summary.data} busy={writing || summarise.isPending} canRun={ollamaReady} onRun={run} />
      ) : (
        <>
          {!writing && !summary.isPending && !summary.error && (
            <div className={styles.empty}>
              <p className="ghostText">
                {transcribed ? "Not yet captured — no summary has been written." : "Not yet captured — the summary is written after the transcript."}
              </p>
              {transcribed && (
                <button type="button" className="btn btnPrimary btnCompact" disabled={!ollamaReady || summarise.isPending} onClick={() => run(false)}>
                  Write summary
                </button>
              )}
            </div>
          )}
          {SKELETON.map((title) => (
            <div key={title} className={styles.section}>
              <h3 className="sectionTitle">{title}</h3>
              <div className={`${styles.slot} ghostCard`} />
            </div>
          ))}
        </>
      )}
    </section>
  );
}

function OllamaCard({ running, model }: { running: boolean; model: string }) {
  const [openFailed, setOpenFailed] = useState(false);
  return (
    <div className={`card ${styles.notice}`}>
      <div>
        <p className={styles.noticeTitle}>{running ? `${model} isn't installed in Ollama` : "Ollama isn't running"}</p>
        <p className="fieldHint">
          {running
            ? `Run “ollama pull ${model}” in Terminal, then come back.`
            : openFailed
              ? "Couldn't open Ollama from /Applications — start it yourself; this page picks it up within seconds."
              : "Summaries are written on this Mac by Qwen through Ollama. Open it and this page picks it up within seconds."}
        </p>
      </div>
      {!running && (
        <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openOllamaApp().then((ok) => setOpenFailed(!ok))}>
          Open Ollama
        </button>
      )}
    </div>
  );
}

function ProgressCard({ meetingId, progress }: { meetingId: string; progress: SummaryProgress | null | undefined }) {
  const label =
    !progress || progress.stage === "queued"
      ? "Waiting for the previous AI job"
      : progress.stage === "merging"
        ? `Merging parts${progress.parts > 1 ? ` · ${progress.part} of ${progress.parts}` : ""}`
        : `Writing${progress.parts > 1 ? ` part ${progress.part} of ${progress.parts}` : ""}`;
  return (
    <div className={`card ${styles.progress}`}>
      <div>
        <p className={styles.noticeTitle}>Summarising{progress && progress.tokens > 0 ? ` · ${progress.tokens} tokens` : "…"}</p>
        <p className="fieldHint">{label}. You'll get a notification when it's ready.</p>
      </div>
      <button type="button" className="btn btnGhost btnCompact" onClick={() => void cancelSummary(meetingId)}>
        Cancel
      </button>
    </div>
  );
}

interface BodyProps {
  slug: string;
  meeting: Meeting;
  view: SummaryView;
  busy: boolean;
  canRun: boolean;
  onRun: (deep: boolean) => void;
}

function SummaryBody({ slug, meeting, view, busy, canRun, onRun }: BodyProps) {
  const copy = useCopyEmail();
  const mail = useOpenInMail();
  const [copied, setCopied] = useState(false);
  const [confirmReplace, setConfirmReplace] = useState<null | boolean>(null); // which re-run is armed
  const edited = !!view.edited_at;

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(t);
  }, [copied]);

  function rerun(deep: boolean) {
    if (edited && confirmReplace !== deep) {
      setConfirmReplace(deep); // first click on an edited summary arms; the second replaces
      return;
    }
    setConfirmReplace(null);
    onRun(deep);
  }

  const announce = copied
    ? "Email copied to the clipboard"
    : confirmReplace !== null
      ? "Your edits will be replaced. Press the button again to confirm."
      : "";
  const actionError = copy.error ?? mail.error;
  return (
    <>
      <div className={styles.bar}>
        <div className={styles.subject}>
          <p className="fieldLabel">Email subject</p>
          <p className="fieldValue">{view.email_subject}</p>
          <p className="fieldHint">
            {view.model ?? "Qwen"} · {edited ? <span className="chip">Edited</span> : "written"} {formatWhen(view.edited_at ?? view.updated_at)}
          </p>
        </div>
        <div className={styles.actions}>
          <button
            type="button"
            className="btn btnPrimary btnCompact"
            disabled={busy || copy.isPending}
            onClick={() => copy.mutate({ slug, meetingId: meeting.id }, { onSuccess: () => setCopied(true) })}
          >
            {copied ? "Copied ✓" : "Copy as email"}
          </button>
          <button type="button" className="btn btnSecondary btnCompact" disabled={busy} onClick={() => mail.mutate({ slug, meetingId: meeting.id })}>
            Open in Mail
          </button>
          <button type="button" className="btn btnGhost btnCompact" disabled={busy || !canRun} onClick={() => rerun(false)}>
            {confirmReplace === false ? "Replace your edits?" : "Re-run"}
          </button>
          <button type="button" className="btn btnGhost btnCompact" aria-describedby="deep-hint" disabled={busy || !canRun} onClick={() => rerun(true)}>
            {confirmReplace === true ? "Replace your edits?" : "Deep re-run"}
          </button>
          <p id="deep-hint" className={`fieldHint ${styles.deepHint}`}>
            Deep re-run lets Qwen think first: more careful, takes several minutes.
          </p>
        </div>
      </div>
      <span className="srOnly" aria-live="polite">
        {announce}
      </span>
      {actionError && (
        <p className={styles.error} role="alert">
          {actionError.message}
        </p>
      )}

      {view.doc.sections.map((section, index) => (
        <SectionBlock
          key={`${view.updated_at}-${index}`} // a new summary resets any open editor
          slug={slug}
          meetingId={meeting.id}
          index={index}
          title={section.title}
          markdown={section.markdown}
          html={view.sections_html[index] ?? ""}
          locked={busy}
        />
      ))}

      <details className={styles.preview}>
        <summary>Email preview</summary>
        <p className={styles.previewSubject}>{view.email_subject}</p>
        <p className={styles.previewBody}>{view.email_md}</p>
      </details>
    </>
  );
}

interface SectionProps {
  slug: string;
  meetingId: string;
  index: number;
  title: string;
  markdown: string;
  html: string;
  locked: boolean;
}

function SectionBlock({ slug, meetingId, index, title, markdown, html, locked }: SectionProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const update = useUpdateSection(slug, meetingId);
  return (
    <div className={styles.section}>
      <div className={styles.sectionHead}>
        <h3 className="sectionTitle">{title}</h3>
        {draft === null && (
          <button type="button" className="btn btnGhost btnCompact" disabled={locked} onClick={() => setDraft(markdown)} aria-label={`Edit ${title}`}>
            Edit
          </button>
        )}
      </div>
      {draft !== null ? (
        <div className={styles.editor}>
          <textarea
            className="input"
            autoFocus
            rows={Math.min(14, Math.max(4, draft.split("\n").length + 1))}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            aria-label={`${title} (Markdown)`}
          />
          {update.error && (
            <p className={styles.error} role="alert">
              {update.error.message}
            </p>
          )}
          <div className={styles.editorActions}>
            <span className="fieldHint">Markdown: “- ” for bullets, **bold**</span>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setDraft(null)}>
              Cancel
            </button>
            <button
              type="button"
              className="btn btnPrimary btnCompact"
              disabled={locked || update.isPending}
              onClick={() => update.mutate({ index, markdown: draft }, { onSuccess: () => setDraft(null) })}
            >
              Save
            </button>
          </div>
        </div>
      ) : html.trim() ? (
        // Rendered in Rust by pulldown-cmark with raw HTML escaped and links/images reduced to
        // text (email::to_html, tested), so it carries no markup the model could inject.
        <div className={styles.prose} dangerouslySetInnerHTML={{ __html: html }} />
      ) : (
        <p className="ghostText">Not yet captured</p>
      )}
    </div>
  );
}
