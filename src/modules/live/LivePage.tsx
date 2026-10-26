import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState, type RefObject } from "react";
import { useMeeting } from "../../kernel/bridge/meetings";
import { useRecorderStatus, type LiveStatus } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace, useSetCurrentWorkspace, useWorkspaces } from "../../kernel/bridge/workspaces";
import { LANGUAGE_LABEL, liveStatusWord } from "../meetings/format";
import TranscriptView, { TranscriptGhost, TranscriptHeads, type TextSize } from "../meetings/TranscriptView";
import StartRecordingDialog from "../recorder/StartRecordingDialog";
import styles from "./LivePage.module.css";

const route = getRouteApi("/shell/live");

/** A−/A+ steps, remembered per Mac. */
const SIZES: TextSize[] = ["md", "lg", "xl"];
const SIZE_KEY = "newdawn.liveTextSize";
/** This close to the bottom still counts as reading the newest line. */
const BOTTOM_SLACK = 32;

// Per-Mac convenience only: storage may be unavailable, so every access is guarded.
function readSize(): TextSize {
  try {
    const saved = localStorage.getItem(SIZE_KEY);
    return SIZES.find((size) => size === saved) ?? "md";
  } catch {
    return "md";
  }
}

function saveSize(size: TextSize) {
  try {
    localStorage.setItem(SIZE_KEY, size);
  } catch {
    // not remembered; the size still applies until the page closes
  }
}

/** Live: Tiếng Việt written down and translated into English as people speak (D-041). A live
 *  session is a recording, so the toolbar pill pauses and stops it; Stop opens the meeting. */
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

  // The meeting page reads the current workspace, so follow the recording there first (as Stop does).
  async function openRecording(slug: string, meetingId: string) {
    if (slug !== workspace?.slug) await switchWorkspace.mutateAsync(slug).catch(() => undefined);
    void navigate({ to: "/meetings/$meetingId", params: { meetingId }, search: { tab: "transcript" } });
  }

  return (
    <>
      {status?.live && slug && meetingId ? (
        <LiveSession
          slug={slug}
          meetingId={meetingId}
          live={status.live}
          paused={status.phase === "paused"}
          eyebrow={workspaces?.find((w) => w.slug === slug)?.name ?? ""}
        />
      ) : (
        <div className="page">
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">Live translation</h1>
          <p className="pageMeta">Tiếng Việt → English as people speak. The session is saved as a meeting.</p>
          {!idle && slug && meetingId && (
            <div className={`card ${styles.running}`}>
              <p>A recording is running without live translation — it's transcribed after Stop.</p>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openRecording(slug, meetingId)}>
                Open the recording
              </button>
            </div>
          )}
          <button type="button" className={`btn btnRecord ${styles.start}`} disabled={!idle || !workspace} onClick={() => void navigate({ to: "/live", search: { start: true } })}>
            <span className="recordDot" />
            Start live translation
          </button>
          <section className="section">
            <TranscriptHeads source={LANGUAGE_LABEL.vi} />
            <TranscriptGhost english />
          </section>
        </div>
      )}
      {/* Outside the branches, so the sheet stays mounted (and closes itself) as the session starts. */}
      {start && <StartRecordingDialog live onClose={() => void navigate({ to: "/live", search: {}, replace: true })} />}
    </>
  );
}

function LiveSession({ slug, meetingId, live, paused, eyebrow }: { slug: string; meetingId: string; live: LiveStatus; paused: boolean; eyebrow: string }) {
  const meeting = useMeeting(slug, meetingId);
  const [size, setSize] = useState(readSize);
  const page = useRef<HTMLDivElement>(null);
  const { following, jump } = useFollow(page);
  const step = (by: number) => {
    const next = SIZES[SIZES.indexOf(size) + by];
    if (!next) return;
    setSize(next);
    saveSize(next);
  };

  return (
    <div className={`page ${styles.session}`} ref={page} tabIndex={-1}>
      <header className={styles.header}>
        <div className={styles.heading}>
          <p className="eyebrow">{eyebrow}</p>
          <h1 className="pageTitle">{meeting.data?.title ?? "Live translation"}</h1>
          <div className={styles.facts}>
            <span className="chip chipPrimary">Tiếng Việt → English</span>
            <span>{liveStatusWord(live, paused)}</span>
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
        <p className={`fieldHint ${styles.dropped}`}>
          The live text skipped about {Math.max(1, Math.round(live.dropped * 0.6))} s it couldn't keep up with. The recording has it: Transcribe again after Stop for the full transcript.
        </p>
      )}
      {meeting.data ? (
        <TranscriptView slug={slug} meeting={meeting.data} textSize={size} />
      ) : (
        <section className="section">
          <TranscriptHeads source={LANGUAGE_LABEL.vi} />
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
