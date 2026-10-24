import { useEffect, useRef, useState } from "react";
import type { SectionKind } from "../../bindings";
import { useSaveNotes, type Meeting } from "../../kernel/bridge/meetings";
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
  const run = (deep: boolean, keepEdits: boolean) => summarise.mutate({ slug, meetingId: meeting.id, deep, keepEdits });

  return (
    <section className="section">
      <NotesCard key={meeting.id} slug={slug} meeting={meeting} />
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
                <button type="button" className="btn btnPrimary btnCompact" disabled={!ollamaReady || summarise.isPending} onClick={() => run(false, true)}>
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
  onRun: (deep: boolean, keepEdits: boolean) => void;
}

function SummaryBody({ slug, meeting, view, busy, canRun, onRun }: BodyProps) {
  const copy = useCopyEmail();
  const mail = useOpenInMail();
  const [copied, setCopied] = useState(false);
  const [pendingRun, setPendingRun] = useState<null | boolean>(null); // deep? while choosing keep/discard
  const edited = !!view.edited_at;
  const editedCount = view.doc.sections.filter((s) => s.edited).length;

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(t);
  }, [copied]);

  function rerun(deep: boolean) {
    if (edited) {
      setPendingRun(deep); // ask: keep or discard the hand edits
      return;
    }
    onRun(deep, true);
  }

  function choose(keepEdits: boolean) {
    if (pendingRun === null) return;
    onRun(pendingRun, keepEdits);
    setPendingRun(null);
  }

  const announce = copied ? "Email copied to the clipboard" : "";
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
            Re-run
          </button>
          <button type="button" className="btn btnGhost btnCompact" aria-describedby="deep-hint" disabled={busy || !canRun} onClick={() => rerun(true)}>
            Deep re-run
          </button>
          <p id="deep-hint" className={`fieldHint ${styles.deepHint}`}>
            Deep re-run lets Qwen think first: more careful, takes several minutes.
          </p>
        </div>
      </div>
      <span className="srOnly" aria-live="polite">
        {announce}
      </span>
      {pendingRun !== null && (
        <div className={`card ${styles.notice}`} role="alertdialog" aria-labelledby="rerun-title">
          <div>
            <p id="rerun-title" className={styles.noticeTitle}>
              {editedCount > 0 ? `You edited ${editedCount === 1 ? "1 section" : `${editedCount} sections`}` : "This summary has older edits"}
            </p>
            <p className="fieldHint">
              {editedCount > 0
                ? "Keep them and let Qwen rewrite the rest, or start over from the transcript."
                : "They were made before edits were tracked per section, so a re-run replaces them."}
            </p>
          </div>
          <div className={styles.choice}>
            <button type="button" className="btn btnGhost btnCompact" autoFocus={editedCount === 0} onClick={() => setPendingRun(null)}>
              Cancel
            </button>
            <button type="button" className={`btn btnCompact ${editedCount > 0 ? "btnSecondary" : "btnPrimary"}`} onClick={() => choose(false)}>
              {editedCount > 0 ? "Discard edits" : "Re-run and replace"}
            </button>
            {editedCount > 0 && (
              <button type="button" className="btn btnPrimary btnCompact" autoFocus onClick={() => choose(true)}>
                Keep my edits
              </button>
            )}
          </div>
        </div>
      )}
      {view.transcript_changed && !busy && (
        <p className={`fieldHint ${styles.stale}`}>
          The transcript was corrected after this summary was written — Re-run to use it (your edited sections are kept).
        </p>
      )}
      {actionError && (
        <p className={styles.error} role="alert">
          {actionError.message}
        </p>
      )}

      {view.doc.sections.map((section, index) => (
        <SectionBlock
          key={`${view.doc.generated_at}-${index}`} // only a new AI summary resets open editors
          slug={slug}
          meetingId={meeting.id}
          index={index}
          kind={section.kind}
          edited={section.edited}
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
  kind: SectionKind;
  edited: boolean;
  title: string;
  markdown: string;
  html: string;
  locked: boolean;
}

// The minutes are rebuilt from these sections' bullets, so the format matters there (D-025).
const FORMAT_HINT: Partial<Record<SectionKind, string>> = {
  key_points: "“- **Topic**”, then “  - point” lines under it",
  decisions: "One “- ” line per decision",
  action_items: "“- **Owner** — task — *due*”, one per line",
  open_questions: "One “- ” line per question",
};

function SectionBlock({ slug, meetingId, index, kind, edited, title, markdown, html, locked }: SectionProps) {
  const [draft, setDraft] = useState<string | null>(null);
  const update = useUpdateSection(slug, meetingId);
  return (
    <div className={styles.section}>
      <div className={styles.sectionHead}>
        <div className={styles.titleRow}>
          <h3 className="sectionTitle">{title}</h3>
          {edited && <span className="chip">Edited</span>}
        </div>
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
            <span className="fieldHint">{FORMAT_HINT[kind] ?? "Markdown: “- ” for bullets, **bold**"}</span>
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

/** The owner's notes: saved as you type, read by the next summary run. */
function NotesCard({ slug, meeting }: { slug: string; meeting: Meeting }) {
  const save = useSaveNotes(slug, meeting.id);
  const [draft, setDraft] = useState(meeting.notes_md ?? "");
  const saved = useRef(meeting.notes_md ?? "");
  const latest = useRef(draft);
  const { mutate } = save;

  // Leaving the tab within the debounce still saves.
  useEffect(
    () => () => {
      if (latest.current !== saved.current) mutate(latest.current);
    },
    [mutate],
  );

  useEffect(() => {
    if (draft === saved.current) return;
    const t = setTimeout(() => {
      saved.current = draft;
      mutate(draft);
    }, 800);
    return () => clearTimeout(t);
  }, [draft, mutate]);

  return (
    <div className={styles.notes}>
      <label className="field">
        <span className="fieldLabel">Your notes</span>
        <textarea
          className="input"
          rows={3}
          value={draft}
          placeholder="Names and spellings, who chaired, what the recording missed…"
          onChange={(e) => {
            latest.current = e.target.value;
            setDraft(e.target.value);
          }}
          onBlur={() => {
            // Clicking Re-run right after typing: save now, not after the debounce.
            if (latest.current !== saved.current) {
              saved.current = latest.current;
              mutate(latest.current);
            }
          }}
        />
      </label>
      <p className="fieldHint">
        {save.error ? save.error.message : save.isPending ? "Saving…" : "Saved on this Mac · the next summary run reads these notes"}
      </p>
    </div>
  );
}
