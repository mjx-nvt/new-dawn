import { useEffect, useRef, useState } from "react";
import { audioUrl, useTranscript, useUpdateSegment, useWaveform, type Meeting, type TranscriptSegment } from "../../kernel/bridge/meetings";
import { formatClock } from "./format";
import styles from "./TranscriptView.module.css";
import WaveformPlayer, { SKIP_MS } from "./WaveformPlayer";

/** Player + timestamped rows; clicking a time seeks the player, the playing row is highlighted,
 *  Edit corrects a line (the next summary run reads the correction). */
export default function TranscriptView({ slug, meeting, startMs }: { slug: string; meeting: Meeting; startMs?: number }) {
  const transcript = useTranscript(slug, meeting.id);
  const player = useRef<HTMLAudioElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [nowMs, setNowMs] = useState(startMs ?? 0);
  const [playing, setPlaying] = useState(false);
  const [rate, setRate] = useState(1);
  const [durationMs, setDurationMs] = useState(meeting.duration_ms ?? 0);
  const [editing, setEditing] = useState<number | null>(null);
  const [playerError, setPlayerError] = useState<string | null>(null);
  const update = useUpdateSegment(slug, meeting.id);
  const waveform = useWaveform(slug, meeting.id, !!meeting.audio_path);
  const segments = transcript.data ?? [];
  const locked = meeting.status === "transcribing";
  const current = segments.find((s) => nowMs >= s.start_ms && nowMs < s.end_ms) ?? null;

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
    <section className="section">
      {meeting.audio_path && (
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
      {segments.length ? (
        <ol className={styles.rows} ref={list}>
          {segments.map((s) => {
            return (
              <li key={s.seq} data-start={s.start_ms} className={current?.seq === s.seq ? `${styles.row} ${styles.playing}` : styles.row}>
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
