import { useEffect, useRef, useState } from "react";
import { audioUrl, useTranscript, useUpdateSegment, type Meeting, type TranscriptSegment } from "../../kernel/bridge/meetings";
import { formatClock } from "./format";
import styles from "./TranscriptView.module.css";

/** Player + timestamped rows; clicking a time seeks the player, the playing row is highlighted,
 *  Edit corrects a line (the next summary run reads the correction). */
export default function TranscriptView({ slug, meeting, startMs }: { slug: string; meeting: Meeting; startMs?: number }) {
  const transcript = useTranscript(slug, meeting.id);
  const player = useRef<HTMLAudioElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [nowMs, setNowMs] = useState(startMs ?? 0);
  const [editing, setEditing] = useState<number | null>(null);
  const update = useUpdateSegment(slug, meeting.id);
  const segments = transcript.data ?? [];
  const locked = meeting.status === "transcribing";

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
    <section className="section">
      {meeting.audio_path && (
        <audio
          ref={player}
          className={styles.player}
          aria-label="Meeting recording"
          controls
          preload="metadata"
          src={audioUrl(meeting.audio_path)}
          onTimeUpdate={(e) => setNowMs(e.currentTarget.currentTime * 1000)}
        />
      )}

      {transcript.error && <p className={styles.error}>{transcript.error.message}</p>}
      {update.error && (
        <p className={styles.error} role="alert">
          {update.error.message}
        </p>
      )}
      {segments.length ? (
        <ol className={styles.rows} ref={list}>
          {segments.map((s) => {
            const playing = nowMs >= s.start_ms && nowMs < s.end_ms;
            return (
              <li key={s.seq} data-start={s.start_ms} className={playing ? `${styles.row} ${styles.playing}` : styles.row}>
                <button
                  type="button"
                  className={styles.time}
                  aria-label={`Play from ${formatClock(s.start_ms)}`}
                  onClick={() => seek(s.start_ms)}
                >
                  {formatClock(s.start_ms)}
                </button>
                {editing === s.seq && !locked ? (
                  <SegmentEditor
                    segment={s}
                    saving={update.isPending}
                    onCancel={() => setEditing(null)}
                    onSave={(text) => update.mutate({ seq: s.seq, text }, { onSuccess: () => setEditing(null) })}
                  />
                ) : (
                  <div className={styles.line}>
                    <p className={styles.text}>{s.text}</p>
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
              </li>
            );
          })}
        </ol>
      ) : (
        <>
          <p className={`ghostText ${styles.empty}`}>Not yet captured</p>
          {[0, 1, 2].map((i) => (
            <div key={i} className={`${styles.ghostRow} ghostCard`} />
          ))}
        </>
      )}
    </section>
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
