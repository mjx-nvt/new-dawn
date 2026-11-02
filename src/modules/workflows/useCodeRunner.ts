import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { answerWorkflowCode, pendingWorkflowCode, refreshAfterRun, type PendingCode } from "../../kernel/bridge/workflows";
import { runCode } from "./codeWorker";

const POLL_MS = 5000;
/** The attempts this window started and hasn't answered: one still here after a reload means the
 *  code took the window down with it, and it isn't run again. */
const STARTED_KEY = "newdawn.workflowCodeStarted";

function started(): string[] {
  try {
    const raw = window.localStorage.getItem(STARTED_KEY);
    const list: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(list) ? list.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
}

function remember(list: string[]) {
  try {
    window.localStorage.setItem(STARTED_KEY, JSON.stringify(list.slice(-50)));
  } catch {
    // Storage off (a private window): no guard against a crash loop, the code still runs.
  }
}

/** Runs the workflows' code steps in this window (D-069): the ones waiting in the current
 *  workspace, every few seconds while the window is visible and when it comes to the front. A
 *  run waits — and its page says so — while the window is closed. Each wait has its own attempt
 *  token, so an answer counts once.
 *  ponytail: the current workspace only; loop over the open ones if code steps run elsewhere. */
export function useCodeRunner(slug: string | undefined) {
  const queryClient = useQueryClient();
  const busy = useRef(new Set<string>());
  useEffect(() => {
    if (!slug) return;
    const answer = async (p: PendingCode) => {
      if (started().includes(p.attempt)) return answerWorkflowCode(slug, p, { ok: false, error: "The code stopped New Dawn's window last time, so it wasn't run again" });
      remember([...started(), p.attempt]);
      const outcome = await runCode(p.code, p.input);
      try {
        return await answerWorkflowCode(slug, p, outcome);
      } finally {
        remember(started().filter((a) => a !== p.attempt));
      }
    };
    const tick = async () => {
      if (document.visibilityState !== "visible") return;
      let pending: PendingCode[];
      try {
        pending = await pendingWorkflowCode(slug);
      } catch {
        return;
      }
      for (const p of pending) {
        if (!p.attempt || busy.current.has(p.attempt)) continue;
        busy.current.add(p.attempt);
        try {
          await answer(p);
          refreshAfterRun(queryClient, slug);
        } catch {
          // The run moved on meanwhile (cancelled, answered): nothing to answer.
        } finally {
          busy.current.delete(p.attempt);
        }
      }
    };
    void tick();
    const timer = window.setInterval(() => void tick(), POLL_MS);
    const onFocus = () => void tick();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", onFocus);
    };
  }, [slug, queryClient]);
}
