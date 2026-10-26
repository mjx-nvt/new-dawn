import { useEffect, useRef, useState, type ReactNode } from "react";
import { audioUrl, useRestoreSegment, useTranscript, useUpdateSegment, useWaveform, type Meeting, type TranscriptSegment } from "../../kernel/bridge/meetings";
import { useRecorderStatus, type LiveStatus } from "../../kernel/bridge/recorder";
import { SparkleIcon } from "../../kernel/shell/icons";
import { columnTitles, directionOf, lastTranslated, layoutLines, type LineLayout, type SpeakerLabel } from "../live/lines";
import { formatClock, pendingEnglish } from "./format";
import styles from "./TranscriptView.module.css";
import WaveformPlayer, { SKIP_MS } from "./WaveformPlayer";

/** The lines' size: the Live page's A−/A+ steps (--t-md, --t-lg, --t-xl). */
export type TextSize = "md" | "lg" | "xl";

/** Why an unclear line has no translation (a saved session's tooltip adds what fixes it). */
const UNCLEAR = "Several voices or a weak signal — not translated.";

/** Player + timestamped rows; clicking a time seeks the player, the playing row is highlighted,
 *  Edit corrects a line (the next summary run reads the correction). A live session's lines
 *  arrive as people speak, with their translation beside them (D-041), and say who spoke and how
 *  clearly they were heard (0.7.5). */
export default function TranscriptView({ slug, meeting, startMs, textSize = "md" }: { slug: string; meeting: Meeting; startMs?: number; textSize?: TextSize }) {
  const { data: recorder } = useRecorderStatus();
  const live = recorder?.meeting_id === meeting.id ? recorder.live : null;
  const transcript = useTranscript(slug, meeting.id, !!live);
  const player = useRef<HTMLAudioElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [nowMs, setNowMs] = useState(startMs ?? 0);
  const [playing, setPlaying] = useState(false);
  const [rate, setRate] = useState(1);
  const [durationMs, setDurationMs] = useState(meeting.duration_ms ?? 0);
  const [editing, setEditing] = useState<number | null>(null);
  const [playerError, setPlayerError] = useState<string | null>(null);
  const update = useUpdateSegment(slug, meeting.id);
  const restore = useRestoreSegment(slug, meeting.id);
  // Still being written: no player, and no waveform cached from half a file.
  const recording = meeting.status === "recording";
  const waveform = useWaveform(slug, meeting.id, !!meeting.audio_path && !recording);
  const segments = transcript.data ?? [];
  const locked = meeting.status === "transcribing" || recording;
  // Said | translation: during a live session and after it, the way its language says.
  const english = !!live || segments.some((s) => s.translation);
  const direction = directionOf(meeting.language);
  // A presenter's paragraphs are a live session's (its lines say their language, translated or
  // not): batch lines stay one row each, as before.
  const fromLive = english || segments.some((s) => s.lang);
  const layout = layoutLines(segments, fromLive ? meeting.format : "discussion");
  const unclear = `${UNCLEAR} Transcribe again${live ? " after Stop" : ""} for a careful pass.`;
  const translatedUpTo = lastTranslated(segments);
  const current = recording ? null : (segments.find((s) => nowMs >= s.start_ms && nowMs < s.end_ms) ?? null);

  // Space plays/pauses, ← → skip 10 s — unless you're typing or on a control that uses them.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (!player.current || e.metaKey || e.ctrlKey || e.altKey) return;
      if (t && (t.closest("input, textarea, select, button, [contenteditable], [role=slider]") || t.isContentEditable)) return;
      if (e.key === " ") {
        if (player.current.paused) void player.current.play();
        else player.current.pause();
      } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        player.current.currentTime = Math.max(0, player.current.currentTime + (e.key === "ArrowLeft" ? -SKIP_MS : SKIP_MS) / 1000);
      } else return;
      e.preventDefault();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  // Arriving from a search hit: scroll to that line and cue the player there (no autoplay).
  const ready = segments.length > 0;
  useEffect(() => {
    if (startMs == null || !ready) return;
    list.current?.querySelector(`[data-start="${startMs}"]`)?.scrollIntoView({ block: "center" });
    if (player.current) player.current.currentTime = startMs / 1000;
  }, [startMs, ready]);

  function seek(ms: number) {
    if (!player.current) return;
    player.current.currentTime = ms / 1000;
    void player.current.play();
  }

  return (
    // Live: new lines and their translation are read out as they arrive (the section is there before the first).
    <section className="section" aria-live={live ? "polite" : undefined}>
      {meeting.audio_path && !recording && (
        <>
          <audio
            ref={player}
            hidden
            preload="metadata"
            src={audioUrl(meeting.audio_path)}
            onTimeUpdate={(e) => setNowMs(e.currentTarget.currentTime * 1000)}
            onLoadedMetadata={(e) => Number.isFinite(e.currentTarget.duration) && setDurationMs(e.currentTarget.duration * 1000)}
            onPlay={() => setPlaying(true)}
            onPause={() => setPlaying(false)}
            onEnded={() => setPlaying(false)}
            onRateChange={(e) => setRate(e.currentTarget.playbackRate)}
            onError={() => setPlayerError("The recording can't be played — the file may have been moved.")}
          />
          <WaveformPlayer
            audio={player}
            peaks={waveform.data?.peaks}
            loading={waveform.isPending}
            durationMs={durationMs || waveform.data?.duration_ms || 0}
            nowMs={nowMs}
            playing={playing}
            rate={rate}
            segment={current}
            onSeek={(ms) => {
              if (player.current) player.current.currentTime = ms / 1000;
              setNowMs(ms);
            }}
          />
          {(playerError ?? waveform.error?.message) && (
            <p className={`fieldHint ${styles.empty}`} role="status">
              {playerError ?? waveform.error?.message}
            </p>
          )}
        </>
      )}

      {!meeting.audio_path && meeting.status === "summarised" && (
        <p className={`fieldHint ${styles.empty}`}>The recording was removed by the retention setting; the transcript stays.</p>
      )}
      {transcript.error && <p className={styles.error}>{transcript.error.message}</p>}
      {update.error && (
        <p className={styles.error} role="alert">
          {update.error.message}
        </p>
      )}
      {segments.some((s) => s.raw_text) && (
        <p className={`fieldHint ${styles.vocab}`}>
          {(() => {
            const n = segments.filter((s) => s.raw_text).length;
            return `${n} ${n === 1 ? "line was" : "lines were"} corrected from the workspace vocabulary. Hover “Corrected” to see what Whisper heard.`;
          })()}
        </p>
      )}
      {english && (
        <TranscriptHeads columns={columnTitles(direction)}>
          <TranslationNote live={live} />
        </TranscriptHeads>
      )}
      {segments.length ? (
        <ol className={textSize === "md" ? styles.rows : `${styles.rows} ${styles[textSize]}`} ref={list}>
          {segments.map((s, i) => {
            return (
              <li key={s.seq} data-start={s.start_ms} className={rowClass(layout[i], english, current?.seq === s.seq)}>
                <Gutter label={layout[i].label}>
                  {recording ? (
                    <span className={styles.time}>{formatClock(s.start_ms)}</span>
                  ) : (
                    <button
                      type="button"
                      className={styles.time}
                      aria-label={`Play from ${formatClock(s.start_ms)}`}
                      onClick={(e) => {
                        seek(s.start_ms);
                        e.currentTarget.blur(); // so Space pauses instead of replaying this line
                      }}
                    >
                      {formatClock(s.start_ms)}
                    </button>
                  )}
                </Gutter>
                {editing === s.seq && !locked ? (
                  <SegmentEditor
                    segment={s}
                    saving={update.isPending}
                    onCancel={() => setEditing(null)}
                    onSave={(text) => update.mutate({ seq: s.seq, text }, { onSuccess: () => setEditing(null) })}
                  />
                ) : (
                  <div className={styles.line}>
                    <Said segment={s} both={direction === "both"} unclear={unclear} />
                    {s.raw_text && (
                      <span className={styles.corrected}>
                        <span className="chip chipPrimary" title={`Whisper heard: ${s.raw_text}`}>
                          Corrected
                        </span>
                        <button type="button" className="btn btnGhost btnCompact" disabled={locked || restore.isPending} title={`Back to: ${s.raw_text} — to stop this correction, remove the spelling in Workspace → Vocabulary`} onClick={() => restore.mutate(s.seq)}>
                          Undo
                        </button>
                      </span>
                    )}
                    <button
                      type="button"
                      className={`btn btnGhost btnCompact ${styles.edit}`}
                      disabled={locked}
                      aria-label={`Edit the line at ${formatClock(s.start_ms)}`}
                      onClick={() => {
                        update.reset();
                        setEditing(s.seq);
                      }}
                    >
                      Edit
                    </button>
                  </div>
                )}
                {english && <Translation segment={s} pending={pendingEnglish(s.seq, translatedUpTo, live)} />}
              </li>
            );
          })}
        </ol>
      ) : (
        <TranscriptGhost english={english} />
      )}
    </section>
  );
}

/** A quick translation's lines: nothing is saved, so no player and no editing. `live` is its
 *  status — the last one seen once it has ended (`running` false). */
export function LiveLines({ lines, live, running, textSize }: { lines: TranscriptSegment[]; live: LiveStatus; running: boolean; textSize: TextSize }) {
  const layout = layoutLines(lines, live.format);
  const translatedUpTo = lastTranslated(lines);
  return (
    <section className="section" aria-live={running ? "polite" : undefined}>
      <TranscriptHeads columns={columnTitles(live.direction)}>
        <TranslationNote live={live} />
      </TranscriptHeads>
      {lines.length ? (
        <ol className={textSize === "md" ? styles.rows : `${styles.rows} ${styles[textSize]}`}>
          {lines.map((s, i) => (
            <li key={s.seq} className={rowClass(layout[i], true)}>
              <Gutter label={layout[i].label}>
                <span className={styles.time}>{formatClock(s.start_ms)}</span>
              </Gutter>
              <div className={styles.line}>
                <Said segment={s} both={live.direction === "both"} unclear={UNCLEAR} />
              </div>
              {/* Ended: a missing translation reads "Not translated" — unless translation was off (the head says why). */}
              <Translation segment={s} pending={pendingEnglish(s.seq, translatedUpTo, running || live.translation_off_reason ? live : null)} />
            </li>
          ))}
        </ol>
      ) : (
        <TranscriptGhost english />
      )}
    </section>
  );
}

function rowClass(layout: LineLayout, bilingual: boolean, playing = false): string {
  return [styles.row, bilingual && styles.bilingual, playing && styles.playing, layout.continues && styles.continued].filter(Boolean).join(" ");
}

/** The time, and who starts speaking there: a discussion's You | Them, at each change. */
function Gutter({ label, children }: { label: SpeakerLabel | null; children: ReactNode }) {
  return (
    <div className={styles.gutter}>
      {children}
      {label && <span className={label === "Them" ? "chip chipPrimary" : "chip"}>{label}</span>}
    </div>
  );
}

/** What was said: the language it was heard in (both ways), the text, and "Unclear" — dotted, not
 *  translated — when Whisper couldn't make it out. */
function Said({ segment: s, both, unclear }: { segment: TranscriptSegment; both: boolean; unclear: string }) {
  const isUnclear = s.clarity === "unclear";
  return (
    <>
      {both && s.lang && (
        <span className={styles.lang} title={s.lang === "vi" ? "Heard in Tiếng Việt" : "Heard in English"}>
          {s.lang.toUpperCase()}
        </span>
      )}
      <p className={isUnclear ? `${styles.text} ${styles.unclear}` : styles.text}>{s.text}</p>
      {isUnclear && (
        <span className="chip" title={unclear}>
          Unclear
        </span>
      )}
    </>
  );
}

/** The translation cell: none for an unclear line, muted and marked when Whisper wasn't sure of
 *  the line, else the translation or what's still to come (`pending`). */
function Translation({ segment: s, pending }: { segment: TranscriptSegment; pending: string | null }) {
  if (s.clarity === "unclear") return <p className={`ghostText ${styles.text}`}>Not translated (unclear)</p>;
  if (!s.translation) return <p className={`ghostText ${styles.text}`}>{pending}</p>;
  if (s.clarity !== "uncertain") return <p className={styles.text}>{s.translation}</p>;
  return (
    <p className={`${styles.text} ${styles.uncertain}`}>
      {s.translation}
      <span className={styles.marker} title="Some words were hard to make out — the translation may be off.">
        May be inaccurate
      </span>
    </p>
  );
}

/** The translation column's note: who translated, or why there's no translation. */
function TranslationNote({ live }: { live: LiveStatus | null }) {
  if (live?.translation_off_reason) return <span className="fieldHint">Off — {live.translation_off_reason}</span>;
  return (
    <span className="fieldHint aiMark">
      <SparkleIcon /> Translated on this Mac{live?.model ? ` by ${live.model}` : ""}
    </span>
  );
}

/** Column heads of a two-language transcript: what was said | its translation, with a note. */
export function TranscriptHeads({ columns: [said, translation], children }: { columns: [string, string]; children?: ReactNode }) {
  return (
    <div className={styles.heads}>
      <span />
      <span className="fieldLabel">{said}</span>
      <span className={styles.headNote}>
        <span className="fieldLabel">{translation}</span>
        {children}
      </span>
    </div>
  );
}

/** Before the first line: "Not yet captured" over three dashed rows, in two columns with English. */
export function TranscriptGhost({ english }: { english: boolean }) {
  return (
    <>
      <p className={`ghostText ${styles.empty}`}>Not yet captured</p>
      {[0, 1, 2].map((i) =>
        english ? (
          <div key={i} className={styles.ghostLine}>
            <span />
            <span className={`${styles.ghostRow} ghostCard`} />
            <span className={`${styles.ghostRow} ghostCard`} />
          </div>
        ) : (
          <div key={i} className={`${styles.ghostRow} ghostCard`} />
        ),
      )}
    </>
  );
}

/** Enter saves, Shift+Enter is a new line, Esc cancels. */
function SegmentEditor({
  segment,
  saving,
  onCancel,
  onSave,
}: {
  segment: TranscriptSegment;
  saving: boolean;
  onCancel: () => void;
  onSave: (text: string) => void;
}) {
  const [draft, setDraft] = useState(segment.text);
  return (
    <div className={styles.editor}>
      <textarea
        className="input"
        autoFocus
        rows={Math.max(2, Math.ceil(draft.length / 90))}
        value={draft}
        aria-label={`Line at ${formatClock(segment.start_ms)}`}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          } else if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            if (draft.trim() && !saving) onSave(draft);
          }
        }}
      />
      <div className={styles.editorActions}>
        <span className="fieldHint">Enter saves · Esc cancels</span>
        <button type="button" className="btn btnGhost btnCompact" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={!draft.trim() || saving} onClick={() => onSave(draft)}>
          Save
        </button>
      </div>
    </div>
  );
}
