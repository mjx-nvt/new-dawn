import { useEffect, useRef, useState } from "react";
import { audioUrl, useTranscript, type Meeting } from "../../kernel/bridge/meetings";
import { formatClock } from "./format";
import styles from "./TranscriptView.module.css";

/** Player + timestamped rows; clicking a row seeks the player, the playing row is highlighted. */
export default function TranscriptView({ slug, meeting, startMs }: { slug: string; meeting: Meeting; startMs?: number }) {
  const transcript = useTranscript(slug, meeting.id);
  const player = useRef<HTMLAudioElement>(null);
  const list = useRef<HTMLOListElement>(null);
  const [nowMs, setNowMs] = useState(startMs ?? 0);
  const segments = transcript.data ?? [];

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
                <p className={styles.text}>{s.text}</p>
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
