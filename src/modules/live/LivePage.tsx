import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState, type RefObject } from "react";
import { useMeeting } from "../../kernel/bridge/meetings";
import { useLastQuick, useLiveLines, useRecorderStatus, type LiveStatus } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace, useSetCurrentWorkspace, useWorkspaces } from "../../kernel/bridge/workspaces";
import { liveStatusWord } from "../meetings/format";
import TranscriptView, { LiveLines, TranscriptGhost, TranscriptHeads, type TextSize } from "../meetings/TranscriptView";
import StartRecordingDialog from "../recorder/StartRecordingDialog";
import { columnTitles, copyText, DIRECTION_LABEL, formatLabel, remember, remembered } from "./lines";
import styles from "./LivePage.module.css";

const route = getRouteApi("/shell/live");

/** A−/A+ steps, remembered per Mac. */
const SIZES: TextSize[] = ["md", "lg", "xl"];
const SIZE_KEY = "newdawn.liveTextSize";
/** This close to the bottom still counts as reading the newest line. */
const BOTTOM_SLACK = 32;
/** Microphone lines dropped as copies of the call before the page asks for headphones. */
const OVERHEARD_HINT = 3;

/** Live: what's said, written down and translated as people speak (D-041). A saved session is a
 *  recording, so the toolbar pill pauses and stops it, and Stop opens the meeting; a quick
 *  translation keeps nothing and stays here, read-only, once it ends. */
export default function LivePage() {
  const { start } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const { data: workspaces } = useWorkspaces();
  const switchWorkspace = useSetCurrentWorkspace();
  const { data: status } = useRecorderStatus();
  const idle = !status || status.phase === "idle";
  const slug = status?.workspace_slug;
  const meetingId = status?.meeting_id;
  const live = status?.live ?? null;
  const paused = status?.phase === "paused";
  // A quick translation that has ended (Stop from anywhere, or it stopped on its own) stays here,
  // read-only, until another session starts.
  const lastQuick = useLastQuick();
  const quick = live?.quick && slug ? { slug, live } : lastQuick;
  const nameOf = (slug: string) => workspaces?.find((w) => w.slug === slug)?.name ?? "";
  const openStart = () => void navigate({ to: "/live", search: { start: true } });

  // The meeting page reads the current workspace, so follow the recording there first (as Stop does).
  async function openRecording(slug: string, meetingId: string) {
    if (slug !== workspace?.slug) await switchWorkspace.mutateAsync(slug).catch(() => undefined);
    void navigate({ to: "/meetings/$meetingId", params: { meetingId }, search: { tab: "transcript" } });
  }

  return (
    <>
      {live && !live.quick && slug && meetingId ? (
        <LiveSession slug={slug} meetingId={meetingId} live={live} running paused={paused} eyebrow={nameOf(slug)} />
      ) : quick ? (
        <LiveSession
          slug={quick.slug}
          live={quick.live}
          running={!!live?.quick}
          paused={paused}
          eyebrow={nameOf(quick.slug)}
          onNewSession={idle && workspace ? openStart : undefined}
        />
      ) : (
        <div className="page">
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">Live translation</h1>
          <p className="pageMeta">
            Writes down what's said and translates it as people speak — Tiếng Việt and English, either way. Save the session as a meeting, or use a quick
            translation that keeps nothing.
          </p>
          {!idle && slug && meetingId && (
            <div className={`card ${styles.running}`}>
              <p>A recording is running without live translation — it's transcribed after Stop.</p>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openRecording(slug, meetingId)}>
                Open the recording
              </button>
            </div>
          )}
          <button type="button" className={`btn btnRecord ${styles.start}`} disabled={!idle || !workspace} onClick={openStart}>
            <span className="recordDot" />
            Start live translation
          </button>
          <section className="section">
            <TranscriptHeads columns={columnTitles("vi_en")} />
            <TranscriptGhost english />
          </section>
        </div>
      )}
      {/* Outside the branches, so the sheet stays mounted (and closes itself) as the session starts. */}
      {start && <StartRecordingDialog live onClose={() => void navigate({ to: "/live", search: {}, replace: true })} />}
    </>
  );
}

function LiveSession({
  slug,
  meetingId,
  live,
  running,
  paused,
  eyebrow,
  onNewSession,
}: {
  slug: string;
  /** None: a quick translation, whose lines are in memory. */
  meetingId?: string;
  live: LiveStatus;
  /** False once a quick translation has ended (its last status stays). */
  running: boolean;
  paused: boolean;
  eyebrow: string;
  /** An ended quick translation's New session; absent while something else records. */
  onNewSession?: () => void;
}) {
  const meeting = useMeeting(meetingId ? slug : undefined, meetingId ?? "");
  const [size, setSize] = useState(() => remembered(SIZE_KEY, SIZES));
  const page = useRef<HTMLDivElement>(null);
  const { following, jump } = useFollow(page);
  const step = (by: number) => {
    const next = SIZES[SIZES.indexOf(size) + by];
    if (!next) return;
    setSize(next);
    remember(SIZE_KEY, next);
  };

  return (
    <div className={`page ${styles.session}`} ref={page} tabIndex={-1}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <p className="eyebrow">{eyebrow}</p>
          <h1 className="pageTitle">{live.quick ? "Quick translation" : (meeting.data?.title ?? "Live translation")}</h1>
          <div className={styles.facts}>
            <span className="chip chipPrimary">{DIRECTION_LABEL[live.direction]}</span>
            <span className="chip">{formatLabel(live.format, live.presenter)}</span>
            {live.quick && <span className="chip">Not saved</span>}
            <span>{running ? liveStatusWord(live, paused) : "Stopped"}</span>
          </div>
        </div>
        <div className={styles.size} role="group" aria-label="Text size">
          <button type="button" className="btn btnGhost btnCompact" aria-label="Smaller text" disabled={size === SIZES[0]} onClick={() => step(-1)}>
            A−
          </button>
          <button type="button" className="btn btnGhost btnCompact" aria-label="Larger text" disabled={size === SIZES[SIZES.length - 1]} onClick={() => step(1)}>
            A+
          </button>
        </div>
      </header>
      {live.dropped > 0 && (
        <p className={`fieldHint ${styles.hint}`}>
          {`The live text skipped about ${Math.max(1, Math.round(live.dropped * 0.6))} s it couldn't keep up with.${live.quick ? "" : " The recording has it: Transcribe again after Stop for the full transcript."}`}
        </p>
      )}
      {/* Always there, so the hint is read out when it appears. */}
      <div role="status">
        {running && live.format === "discussion" && live.overheard >= OVERHEARD_HINT && (
          <p className={`fieldHint ${styles.hint}`}>Your microphone hears the call — use headphones.</p>
        )}
      </div>
      {!meetingId ? (
        <QuickLines live={live} running={running} textSize={size} onNewSession={onNewSession} />
      ) : meeting.data ? (
        <TranscriptView slug={slug} meeting={meeting.data} textSize={size} />
      ) : (
        <section className="section">
          <TranscriptHeads columns={columnTitles(live.direction)} />
          <TranscriptGhost english />
        </section>
      )}
      <div className={styles.latestBar}>
        {!following && (
          <button type="button" className={`btn btnSecondary btnCompact ${styles.latest}`} aria-label="Jump to the latest line" onClick={jump}>
            ↓ Latest
          </button>
        )}
      </div>
    </div>
  );
}

/** A quick translation's lines, from memory; once it has ended, Copy all and New session. */
function QuickLines({ live, running, textSize, onNewSession }: { live: LiveStatus; running: boolean; textSize: TextSize; onNewSession?: () => void }) {
  const lines = useLiveLines(running);
  const all = lines.data ?? [];
  const [copied, setCopied] = useState<boolean | null>(null); // false: copying failed
  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(null), 2000);
    return () => clearTimeout(t);
  }, [copied]);

  // Nothing else keeps these lines, so a failed copy says so.
  async function copyAll() {
    try {
      await navigator.clipboard.writeText(copyText(all, live.format));
      setCopied(true);
    } catch {
      setCopied(false);
    }
  }

  // After the lines, where the follow logic keeps the reader.
  return (
    <>
      {lines.error && <p className="formError">{lines.error.message}</p>}
      <LiveLines lines={all} live={live} running={running} textSize={textSize} />
      {!running && (
        <div className={styles.ended}>
          <div className={styles.actions}>
            <button type="button" className="btn btnSecondary btnCompact" disabled={!all.length} onClick={() => void copyAll()}>
              {copied ? "Copied" : "Copy all"}
            </button>
            <button type="button" className="btn btnRecord btnCompact" disabled={!onNewSession} onClick={onNewSession}>
              <span className="recordDot" />
              New session
            </button>
          </div>
          <p className="fieldHint">Not saved — nothing of this session is kept after you quit New Dawn or start another one.</p>
          {copied === false && (
            <p className="formError" role="alert">
              Couldn't copy — select the lines and press ⌘C.
            </p>
          )}
          <span className="srOnly" aria-live="polite">
            {copied ? "Both columns copied to the clipboard" : ""}
          </span>
        </div>
      )}
    </>
  );
}

/** Keeps the newest line in view while the reader is at the bottom of the page (the content
 *  panel scrolls); scrolling up stops that until they scroll back down or `jump`. */
function useFollow(page: RefObject<HTMLDivElement | null>) {
  const [following, setFollowing] = useState(true);
  const follow = useRef(true);
  const jumping = useRef(false); // a smooth jump is on its way down: don't count its steps

  useEffect(() => {
    const content = page.current;
    const root = content?.closest<HTMLElement>(".content-root");
    if (!content || !root) return;
    const onScroll = () => {
      const atBottom = root.scrollHeight - root.scrollTop - root.clientHeight < BOTTOM_SLACK;
      if (atBottom) jumping.current = false;
      if (jumping.current) return;
      follow.current = atBottom;
      setFollowing(atBottom);
    };
    const onWheel = () => {
      jumping.current = false; // the reader took over mid-jump
    };
    // New lines, their English and a bigger text size all make the page taller.
    const grown = new ResizeObserver(() => {
      if (follow.current) root.scrollTop = root.scrollHeight;
    });
    grown.observe(content);
    root.addEventListener("scroll", onScroll, { passive: true });
    root.addEventListener("wheel", onWheel, { passive: true });
    return () => {
      grown.disconnect();
      root.removeEventListener("scroll", onScroll);
      root.removeEventListener("wheel", onWheel);
    };
  }, [page]);

  const jump = () => {
    const root = page.current?.closest<HTMLElement>(".content-root");
    if (!root) return;
    // The button goes away once following: keep keyboard focus in the page, not on <body>.
    page.current?.focus({ preventScroll: true });
    jumping.current = true;
    follow.current = true;
    setFollowing(true);
    const instant = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    root.scrollTo({ top: root.scrollHeight, behavior: instant ? "auto" : "smooth" });
  };
  return { following, jump };
}
