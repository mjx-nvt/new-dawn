import { useEffect, useRef, type RefObject } from "react";
import { PauseIcon, PlayIcon, SkipBackIcon, SkipForwardIcon } from "../../kernel/shell/icons";
import { formatClock } from "./format";
import styles from "./WaveformPlayer.module.css";

export const SKIP_MS = 10_000;
const RATES = [1, 1.5, 2] as const;
const BAR = 2;
const GAP = 1;

/** The recording as a waveform you can click into, with play/pause, ±10 s and speed. The
 *  `<audio>` element itself belongs to the caller (it also drives the transcript highlight). */
export default function WaveformPlayer({
  audio,
  peaks,
  loading,
  durationMs,
  nowMs,
  playing,
  rate,
  segment,
  onSeek,
}: {
  audio: RefObject<HTMLAudioElement | null>;
  /** One 0–255 peak per slice of the file; undefined while loading or when decoding failed. */
  peaks?: number[];
  loading: boolean;
  durationMs: number;
  nowMs: number;
  playing: boolean;
  rate: number;
  /** The transcript line being played, shown as a band behind the bars. */
  segment: { start_ms: number; end_ms: number } | null;
  onSeek: (ms: number) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const duration = Math.max(durationMs, 1);

  const toggle = () => {
    const el = audio.current;
    if (!el) return;
    if (el.paused) void el.play();
    else el.pause();
  };
  const skip = (delta: number) => onSeek(Math.min(duration, Math.max(0, nowMs + delta)));
  const seekAt = (clientX: number) => {
    const box = canvas.current?.getBoundingClientRect();
    if (!box || box.width === 0) return;
    onSeek(((clientX - box.left) / box.width) * duration);
  };

  // Redraw on every change (a few times a second while playing; 400 bars at most — cheap).
  // The latest drawing is kept in a ref so the observer below is created once.
  const draw = useRef(() => {});
  draw.current = () => {
    const el = canvas.current;
    if (!el) return;
    {
      const dpr = window.devicePixelRatio || 1;
      const w = el.clientWidth;
      const h = el.clientHeight;
      if (w === 0 || h === 0) return;
      el.width = Math.round(w * dpr);
      el.height = Math.round(h * dpr);
      const ctx = el.getContext("2d");
      if (!ctx) return;
      ctx.scale(dpr, dpr);
      const style = getComputedStyle(el);
      const primary = style.getPropertyValue("--primary").trim();
      const muted = style.getPropertyValue("--ink-3").trim();
      ctx.clearRect(0, 0, w, h);
      if (segment) {
        ctx.fillStyle = primary;
        ctx.globalAlpha = 0.14;
        const x0 = (segment.start_ms / duration) * w;
        const x1 = (segment.end_ms / duration) * w;
        ctx.fillRect(x0, 0, Math.max(x1 - x0, BAR), h);
        ctx.globalAlpha = 1;
      }
      const bars = Math.max(1, Math.floor(w / (BAR + GAP)));
      const source = peaks && peaks.length ? peaks : null;
      const played = (nowMs / duration) * w;
      for (let i = 0; i < bars; i++) {
        let amp = 0.06;
        if (source) {
          const from = Math.floor((i * source.length) / bars);
          const to = Math.max(from + 1, Math.floor(((i + 1) * source.length) / bars));
          let peak = 0;
          for (let j = from; j < to; j++) peak = Math.max(peak, source[j]);
          amp = Math.max(0.06, (peak / 255) ** 1.4); // speech peaks bunch up near the top; the curve spreads them
        }
        const x = i * (BAR + GAP);
        const bh = Math.max(2, amp * h * 0.92);
        const isPlayed = x + BAR <= played;
        ctx.fillStyle = isPlayed ? primary : muted;
        ctx.globalAlpha = isPlayed ? 1 : 0.45;
        ctx.fillRect(x, (h - bh) / 2, BAR, bh);
      }
      ctx.globalAlpha = 1;
    }
  };
  useEffect(() => draw.current(), [peaks, nowMs, duration, segment]);
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    const redraw = () => draw.current();
    const observer = new ResizeObserver(redraw);
    observer.observe(el);
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    scheme.addEventListener("change", redraw);
    return () => {
      observer.disconnect();
      scheme.removeEventListener("change", redraw);
    };
  }, []);

  return (
    <div className={`glass ${styles.player}`}>
      <button type="button" className={`btn btnPrimary ${styles.play}`} aria-label={playing ? "Pause" : "Play"} onClick={toggle}>
        {playing ? <PauseIcon /> : <PlayIcon />}
      </button>
      <canvas
        ref={canvas}
        className={styles.wave}
        role="slider"
        tabIndex={0}
        aria-label="Recording position"
        aria-valuemin={0}
        aria-valuemax={Math.round(duration / 1000)}
        aria-valuenow={Math.round(nowMs / 1000)}
        aria-valuetext={`${formatClock(nowMs)} of ${formatClock(duration)}`}
        aria-busy={loading || undefined}
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          seekAt(e.clientX);
        }}
        onPointerMove={(e) => e.buttons === 1 && seekAt(e.clientX)}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft") skip(-5000);
          else if (e.key === "ArrowRight") skip(5000);
          else if (e.key === "Home") onSeek(0);
          else if (e.key === "End") onSeek(duration);
          else if (e.key === " ") toggle();
          else return;
          e.preventDefault();
        }}
      />
      <span className={styles.time}>
        {formatClock(nowMs)} <span className={styles.timeTotal}>/ {formatClock(duration)}</span>
      </span>
      <div className={styles.skips}>
        <button type="button" className="btn btnGhost btnCompact" aria-label="Back 10 seconds" onClick={() => skip(-SKIP_MS)}>
          <SkipBackIcon />
          10
        </button>
        <button type="button" className="btn btnGhost btnCompact" aria-label="Forward 10 seconds" onClick={() => skip(SKIP_MS)}>
          10
          <SkipForwardIcon />
        </button>
      </div>
      <div className="segmented" role="group" aria-label="Speed">
        {RATES.map((r) => (
          <button
            key={r}
            type="button"
            aria-pressed={rate === r}
            onClick={() => {
              if (audio.current) audio.current.playbackRate = r;
            }}
          >
            {r}×
          </button>
        ))}
      </div>
    </div>
  );
}
