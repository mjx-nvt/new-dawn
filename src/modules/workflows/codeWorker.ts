/** A code step's JavaScript, run in a Web Worker made for it (D-069): Twenty's
 *  `export const main = async (params) => { … return { … } }`, given the step's inputs, its
 *  return value the output. The worker has no DOM and no way out — the window's policy allows no
 *  connection but the app's own, and the worker loses fetch, XMLHttpRequest, WebSocket,
 *  EventSource and importScripts before the code runs — and it is stopped after 10 s. */

const TIMEOUT_MS = 10_000;
// A nested worker would get a fresh scope with fetch: Worker goes too. The window's content
// policy (no connection but the app's own, no eval) is the wall; this is the fence before it.
const CUT_OFF = ["fetch", "XMLHttpRequest", "WebSocket", "EventSource", "importScripts", "indexedDB", "caches", "Worker", "SharedWorker", "WebTransport"];

export type CodeOutcome = { ok: true; output: unknown } | { ok: false; error: string };

/** The step's code as a classic script: `export` has no place in a worker made from text. */
function asScript(code: string): string {
  return code
    .replace(/export\s+const\s+main\s*=/, "const main =")
    .replace(/export\s+(async\s+)?function\s+main/, "$1function main")
    .replace(/export\s+default\s+/, "const main = ");
}

export function runCode(code: string, input: unknown): Promise<CodeOutcome> {
  const source = `"use strict";
for (const name of ${JSON.stringify(CUT_OFF)}) { try { Object.defineProperty(self, name, { value: undefined }); } catch (e) {} }
${asScript(code)}
self.onmessage = async (event) => {
  try {
    if (typeof main !== "function") throw new Error("The code defines no main function");
    const out = await main(event.data);
    self.postMessage({ ok: true, output: JSON.parse(JSON.stringify(out === undefined ? null : out)) });
  } catch (error) {
    self.postMessage({ ok: false, error: String(error && error.message ? error.message : error) });
  }
};`;
  const url = URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  return new Promise((resolve) => {
    let worker: Worker;
    try {
      worker = new Worker(url);
    } catch (e) {
      URL.revokeObjectURL(url);
      resolve({ ok: false, error: e instanceof Error ? e.message : String(e) });
      return;
    }
    const finish = (outcome: CodeOutcome) => {
      window.clearTimeout(timer);
      worker.terminate();
      URL.revokeObjectURL(url);
      resolve(outcome);
    };
    const timer = window.setTimeout(() => finish({ ok: false, error: "The code ran longer than 10 s and was stopped" }), TIMEOUT_MS);
    // Only main's own answer counts: code that posts a message itself isn't a result.
    worker.onmessage = (event: MessageEvent<unknown>) => {
      const d = event.data as Partial<CodeOutcome> | null;
      finish(d && typeof d === "object" && typeof d.ok === "boolean" ? (d as CodeOutcome) : { ok: false, error: "The code answered without main's result" });
    };
    worker.onerror = (event) => {
      event.preventDefault();
      finish({ ok: false, error: event.message || "The code doesn't run" });
    };
    worker.postMessage(input);
  });
}
