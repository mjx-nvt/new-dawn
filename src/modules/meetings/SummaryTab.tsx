import { useEffect, useRef, useState } from "react";
import type { NameSuspect, SectionKind } from "../../bindings";
import { useSaveNotes, type Meeting } from "../../kernel/bridge/meetings";
import {
  cancelSummary,
  openOllamaApp,
  useCopyEmail,
  useDismissNames,
  useOllamaStatus,
  useOpenInMail,
  useRewriteEmail,
  useSummarise,
  useSummary,
  useSummaryError,
  useSummaryProgress,
  useUpdateSection,
  type SummaryProgress,
  type SummaryView,
} from "../../kernel/bridge/summary";
import { SparkleIcon } from "../../kernel/shell/icons";
import MenuButton from "../../kernel/shell/MenuButton";
import { SummaryBar } from "./AiProgress";
import AttendeesBlock from "./AttendeesBlock";
import NamesSheet from "./NamesSheet";
import { formatWhen } from "./format";
import styles from "./SummaryTab.module.css";

// Until meeting types drive the structure (P1.4), the same sections the backend uses by default.
const SKELETON = ["Overview", "Key points", "Decisions", "Action items", "Next steps", "Open questions"];

/** Executive summary: your notes on top (pinned while you scroll), then the summary with its
 *  highlights and sections; the email and attendees on the right, which stay in view too. */
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
  const busy = writing || summarise.isPending;
  const view = summary.data;

  return (
    <div className={styles.layout}>
      <div className={styles.main}>
        {ollama.data && !ollamaReady && !writing && <OllamaCard running={ollama.data.running} model={ollama.data.model} />}
        {writing && <ProgressCard meetingId={meeting.id} progress={progress} />}
        {error && (
          <p className={styles.error} role="alert">
            {error.message}
          </p>
        )}
        <div className={`card ${styles.asideCard} ${styles.notesPinned}`}>
          <NotesCard key={meeting.id} slug={slug} meeting={meeting} canRun={!!view && ollamaReady && !busy} onRerun={() => run(false, true)} />
        </div>
        {view ? (
          <Sections slug={slug} meeting={meeting} view={view} busy={busy} canRun={ollamaReady} onRun={run} />
        ) : (
          <>
            {!writing && !summary.isPending && !summary.error && (
              <div className={`card ${styles.empty}`}>
                <div>
                  <p className={styles.emptyTitle}>{transcribed ? "No summary yet" : "Transcript first"}</p>
                  <p className="fieldHint">
                    {transcribed ? "Qwen reads the transcript and your notes, then writes it. About a minute." : "The summary is written once the transcript is ready."}
                  </p>
                </div>
                {transcribed && (
                  <button type="button" className="btn btnPrimary btnAi" aria-busy={busy || undefined} disabled={!ollamaReady || summarise.isPending} onClick={() => run(false, true)}>
                    <SparkleIcon />
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
      </div>

      <aside className={styles.aside}>
        <EmailCard slug={slug} meeting={meeting} view={view ?? undefined} busy={busy} />
        <div className={`card ${styles.asideCard}`}>
          <AttendeesBlock key={view?.doc.generated_at ?? "none"} slug={slug} meetingId={meeting.id} attendees={view?.doc.attendees ?? []} locked={busy || !view} />
        </div>
      </aside>
    </div>
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
            ? "Download it in Settings → Models."
            : openFailed
              ? "Not found in Applications — start it yourself; this page picks it up within seconds."
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
  return (
    <div className={`card ${styles.notice} ${styles.progressCard}`}>
      <div className={styles.progressBody}>
        <p className={`${styles.noticeTitle} aiMark`}>
          <SparkleIcon />
          Summarising
        </p>
        {progress ? <SummaryBar progress={progress} /> : <p className="fieldHint">Starting…</p>}
        <p className="fieldHint">You'll get a notification when it's ready; you can leave this page.</p>
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

/** The sections, each editable, with the re-run choice when something was edited. */
function Sections({ slug, meeting, view, busy, canRun, onRun }: BodyProps) {
  const [pendingRun, setPendingRun] = useState<null | boolean>(null); // deep? while choosing keep/discard
  const edited = !!view.edited_at;
  const editedCount = view.doc.sections.filter((s) => s.edited).length;

  function rerun(deep: boolean) {
    if (edited) setPendingRun(deep);
    else onRun(deep, true);
  }

  function choose(keepEdits: boolean) {
    if (pendingRun === null) return;
    onRun(pendingRun, keepEdits);
    setPendingRun(null);
  }

  return (
    <>
      <div className={styles.sectionsHead}>
        <p className="fieldHint aiMark">
          <SparkleIcon />
          {edited ? `Edited ${formatWhen(view.edited_at ?? view.updated_at)} · written by ${view.model ?? "Qwen"}` : `Written by ${view.model ?? "Qwen"} · ${formatWhen(view.updated_at)}`}
        </p>
        <MenuButton
          className="btn btnCompact btnAi"
          label="Re-run options"
          items={[
            { label: "Re-run", onSelect: () => rerun(false), disabled: busy || !canRun },
            { label: "Deep re-run (thinks first, slower)", onSelect: () => rerun(true), disabled: busy || !canRun },
          ]}
        >
          <SparkleIcon />
          Re-run ▾
        </MenuButton>
      </div>

      {pendingRun !== null && (
        <div className={`card ${styles.notice}`} role="alertdialog" aria-labelledby="rerun-title">
          <div>
            <p id="rerun-title" className={styles.noticeTitle}>
              {editedCount > 0 ? `Keep your ${editedCount === 1 ? "edit" : `${editedCount} edits`}?` : "This summary has older edits"}
            </p>
            <p className="fieldHint">
              {editedCount > 0 ? "Kept sections stay as you wrote them; Qwen rewrites the rest." : "They can't be kept section by section, so a re-run replaces them."}
            </p>
          </div>
          <div className={styles.choice}>
            <button type="button" className="btn btnGhost btnCompact" autoFocus={editedCount === 0} onClick={() => setPendingRun(null)}>
              Cancel
            </button>
            <button type="button" className={`btn btnCompact ${editedCount > 0 ? "btnDanger" : "btnPrimary btnAi"}`} onClick={() => choose(false)}>
              {editedCount === 0 && <SparkleIcon />}
              {editedCount > 0 ? "Discard edits" : "Re-run and replace"}
            </button>
            {editedCount > 0 && (
              <button type="button" className="btn btnPrimary btnCompact btnAi" autoFocus onClick={() => choose(true)}>
                <SparkleIcon />
                Keep my edits
              </button>
            )}
          </div>
        </div>
      )}
      {view.transcript_changed && !busy && <p className={`fieldHint ${styles.stale}`}>The transcript changed since — re-run to use it; your edits are kept.</p>}

      {view.naming && !busy && <NamesCard slug={slug} meetingId={meeting.id} suspects={view.naming.suspects} aiChecked={view.naming.ai_checked} />}

      {(view.doc.highlights.length > 0 || meeting.notes_md?.trim()) && (
        <div className={`${styles.section} ${styles.highlights}`}>
          <p className="eyebrow">From your notes</p>
          <h3 className="sectionTitle">Highlights</h3>
          {view.doc.highlights.length > 0 ? (
            <ul className={styles.highlightList}>
              {view.doc.highlights.map((h, i) => (
                <li key={i}>{h}</li>
              ))}
            </ul>
          ) : (
            <p className="ghostText">Not yet captured — re-run with your notes</p>
          )}
        </div>
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
    </>
  );
}

/** The naming agent's doubts: how many, open the sheet, or dismiss. */
function NamesCard({ slug, meetingId, suspects, aiChecked }: { slug: string; meetingId: string; suspects: NameSuspect[]; aiChecked: boolean }) {
  const [open, setOpen] = useState(false);
  const dismiss = useDismissNames(slug, meetingId);
  const n = suspects.length;
  return (
    <div className={`card ${styles.notice}`} role="status">
      <div>
        <p className={`${styles.noticeTitle} aiMark`}>
          <SparkleIcon />
          {n === 1 ? "1 name to confirm" : `${n} names to confirm`}
        </p>
        <p className="fieldHint">
          {suspects
            .slice(0, 4)
            .map((s) => (s.suggestion !== s.heard_as ? `${s.heard_as} → ${s.suggestion}` : s.heard_as))
            .join(" · ")}
          {n > 4 ? " · …" : ""}
          {aiChecked ? "" : " (checked against the people memory only)"}
        </p>
      </div>
      <div className={styles.choice}>
        <button type="button" className="btn btnGhost btnCompact" disabled={dismiss.isPending} onClick={() => dismiss.mutate()}>
          Dismiss
        </button>
        <button type="button" className="btn btnPrimary btnCompact" onClick={() => setOpen(true)}>
          Review names
        </button>
      </div>
      {open && <NamesSheet slug={slug} meetingId={meetingId} suspects={suspects} onClose={() => setOpen(false)} />}
    </div>
  );
}

/** The summary as an email: subject, copy, open in Mail, and a preview on request. */
function EmailCard({ slug, meeting, view, busy }: { slug: string; meeting: Meeting; view: SummaryView | undefined; busy: boolean }) {
  const copy = useCopyEmail();
  const mail = useOpenInMail();
  const rewrite = useRewriteEmail(slug, meeting.id);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(t);
  }, [copied]);
  const actionError = copy.error ?? mail.error ?? rewrite.error;
  return (
    <div className={`card ${styles.asideCard}`}>
      <p className={styles.asideTitle}>Email</p>
      {view ? <p className={styles.subject}>{view.email_subject}</p> : <p className="ghostText">Not yet captured</p>}
      <p className="fieldHint">{view?.edited_at ? "Edited since the email was written — Rewrite refreshes it." : "A cover note: the minutes attached, the key points in detail. Open in Mail attaches the latest export."}</p>
      <div className={styles.asideActions}>
        <button type="button" className="btn btnPrimary btnCompact" disabled={!view || busy || copy.isPending} onClick={() => copy.mutate({ slug, meetingId: meeting.id }, { onSuccess: () => setCopied(true) })}>
          {copied ? "Copied" : "Copy as email"}
        </button>
        <button type="button" className="btn btnSecondary btnCompact" disabled={!view || busy} onClick={() => mail.mutate({ slug, meetingId: meeting.id })}>
          Open in Mail
        </button>
        <button type="button" className="btn btnCompact btnAi" aria-busy={rewrite.isPending || undefined} disabled={!view || busy || rewrite.isPending} title="The email agent writes the cover note again from the summary as it is now" onClick={() => rewrite.mutate()}>
          <SparkleIcon />
          {rewrite.isPending ? "Writing…" : "Rewrite"}
        </button>
      </div>
      <span className="srOnly" aria-live="polite">
        {copied ? "Email copied to the clipboard" : ""}
      </span>
      {actionError && (
        <p className={styles.error} role="alert">
          {actionError.message}
        </p>
      )}
      {view && (
        <details className={styles.preview}>
          <summary>Preview</summary>
          <p className={styles.previewBody}>{view.email_md}</p>
        </details>
      )}
    </div>
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
          <button type="button" className={`btn btnGhost btnCompact ${styles.editButton}`} disabled={locked} onClick={() => setDraft(markdown)} aria-label={`Edit ${title}`}>
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
            <button type="button" className="btn btnPrimary btnCompact" disabled={locked || update.isPending} onClick={() => update.mutate({ index, markdown: draft }, { onSuccess: () => setDraft(null) })}>
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
function NotesCard({ slug, meeting, canRun, onRerun }: { slug: string; meeting: Meeting; canRun: boolean; onRerun: () => void }) {
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
    <>
      <p className={styles.asideTitle}>Your notes</p>
      <textarea
        className="input"
        rows={4}
        value={draft}
        aria-label="Your notes"
        placeholder="Names, who chaired, what the recording missed…"
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
      <p className="fieldHint">{save.error ? save.error.message : save.isPending ? "Saving…" : "What you note here is highlighted in the summary."}</p>
      <button
        type="button"
        className="btn btnCompact btnAi"
        disabled={!canRun || !draft.trim()}
        title="Qwen rewrites the summary with your notes as the highlights; your edited sections are kept"
        onClick={() => {
          if (latest.current !== saved.current) {
            saved.current = latest.current;
            mutate(latest.current);
          }
          onRerun();
        }}
      >
        <SparkleIcon />
        Re-run with my notes
      </button>
    </>
  );
}
