import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useWorkspacesSync } from "../../kernel/bridge/events";
import { testNotification, useFinishOnboarding, useOllamaPull } from "../../kernel/bridge/onboarding";
import { openPrivacySettings, useMicrophonePermission, useRequestMicrophone } from "../../kernel/bridge/recorder";
import { openOllamaApp, useOllamaStatus } from "../../kernel/bridge/summary";
import { useWorkspaces } from "../../kernel/bridge/workspaces";
import { SpeechModels } from "../models/ModelsPanel";
import ConvertersPanel from "../workspace-settings/ConvertersPanel";
import WorkspacesPanel from "../workspace-settings/WorkspacesPanel";
import styles from "./WelcomePage.module.css";

export const WELCOME_STEPS = [
  { id: "welcome", label: "Welcome" },
  { id: "permissions", label: "Permissions" },
  { id: "ollama", label: "Text model" },
  { id: "whisper", label: "Speech model" },
  { id: "pdf", label: "PDF" },
  { id: "workspace", label: "Workspace" },
  { id: "done", label: "Done" },
] as const;

export type WelcomeStep = (typeof WELCOME_STEPS)[number]["id"];

const route = getRouteApi("/welcome");

/** First-run setup: every step can be skipped; New Dawn → Run Setup… shows it again. */
export default function WelcomePage() {
  const { step } = route.useSearch();
  const navigate = useNavigate();
  const finish = useFinishOnboarding();
  useWorkspacesSync();
  const index = WELCOME_STEPS.findIndex((s) => s.id === step);
  const go = (to: number) => void navigate({ to: "/welcome", search: { step: WELCOME_STEPS[to].id }, replace: true });
  const close = () => finish.mutate(true, { onSuccess: () => void navigate({ to: "/meetings", search: {}, replace: true }) }); // Back never returns here

  return (
    <div className={styles.window}>
      <header className={styles.strip} data-tauri-drag-region="deep">
        <ol className={styles.steps}>
          {WELCOME_STEPS.map((s, i) => (
            <li key={s.id} className={i === index ? styles.current : i < index ? styles.past : undefined}>
              {s.label}
            </li>
          ))}
        </ol>
      </header>
      <main className={`content-root ${styles.content}`}>
        <div className={styles.page}>
          <StepBody step={step} />
          <footer className={styles.footer}>
            {index > 0 && step !== "done" && (
              <button type="button" className="btn btnGhost" onClick={() => go(index - 1)}>
                Back
              </button>
            )}
            <span className={styles.spacer} />
            {step !== "done" && (
              <button type="button" className="btn btnGhost" disabled={finish.isPending} onClick={close}>
                Skip setup
              </button>
            )}
            {step === "done" ? (
              <button type="button" className="btn btnPrimary" autoFocus disabled={finish.isPending} onClick={close}>
                Open New Dawn
              </button>
            ) : (
              <button type="button" className="btn btnPrimary" onClick={() => go(index + 1)}>
                {step === "welcome" ? "Get started" : "Continue"}
              </button>
            )}
          </footer>
          {finish.error && (
            <p className="formError" role="alert">
              {finish.error.message}
            </p>
          )}
        </div>
      </main>
    </div>
  );
}

function StepBody({ step }: { step: WelcomeStep }) {
  switch (step) {
    case "welcome":
      return (
        <Intro title="Welcome to New Dawn" eyebrow="Your meeting machine">
          Record a call, get the full transcript, an executive summary ready to email, and minutes in your company's Word
          template. Everything runs on this Mac — no bot joins your calls, and nothing is sent to the cloud. A few minutes of
          setup and you're ready.
        </Intro>
      );
    case "permissions":
      return <PermissionsStep />;
    case "ollama":
      return <OllamaStep />;
    case "whisper":
      return (
        <>
          <Intro title="Speech model" eyebrow="Whisper">
            Whisper turns recordings into text on this Mac. Download large-v3-turbo (about 1.6 GB, once); you can keep using
            the app while it downloads.
          </Intro>
          <SpeechModels />
        </>
      );
    case "pdf":
      return (
        <>
          <Intro title="PDF converter" eyebrow="Optional">
            Minutes are Word documents. To also make a PDF, New Dawn asks an app you already have — Microsoft Word,
            LibreOffice or Pages. Nothing to do here if one is found.
          </Intro>
          <ConvertersPanel />
        </>
      );
    case "workspace":
      return <WorkspaceStep />;
    case "done":
      return (
        <Intro title="You're set" eyebrow="Ready">
          Start a recording with ⌘R or the ● Record button in the toolbar when your call begins, and stop it when it ends. The
          transcript and summary follow on their own — you'll get a notification. Settings (⌘,) has everything from this
          setup, and New Dawn → Run Setup… brings it back.
        </Intro>
      );
  }
}

function Intro({ title, eyebrow, children }: { title: string; eyebrow: string; children: React.ReactNode }) {
  return (
    <div className={styles.intro}>
      <p className="eyebrow">{eyebrow}</p>
      <h1 className="pageTitle">{title}</h1>
      <p className={styles.lead}>{children}</p>
    </div>
  );
}

function PermissionsStep() {
  const mic = useMicrophonePermission();
  const request = useRequestMicrophone();
  const [notified, setNotified] = useState(false);
  return (
    <>
      <Intro title="Permissions" eyebrow="Asked once">
        macOS asks before an app can hear you or notify you. New Dawn keeps the audio on this Mac.
      </Intro>
      <div className={styles.rows}>
        <Row title="Microphone" hint="Your side of the call.">
          {mic.data === "granted" ? (
            <span className="chip chipBrand">Allowed</span>
          ) : mic.data === "not_determined" ? (
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => request.mutate()}>
              Allow…
            </button>
          ) : (
            <button type="button" className="btn btnGhost btnCompact" onClick={() => void openPrivacySettings(false)}>
              Open Settings
            </button>
          )}
        </Row>
        <Row title="Call audio" hint="The other side of the call. macOS asks the first time you record — choose Allow.">
          <span className="chip">At first recording</span>
        </Row>
        <Row title="Notifications" hint="Transcript and summary ready, recording started and stopped.">
          <button
            type="button"
            className="btn btnSecondary btnCompact"
            onClick={() => void testNotification().then(() => setNotified(true))}
          >
            {notified ? "Sent — allow it if macOS asks" : "Send a test"}
          </button>
        </Row>
      </div>
    </>
  );
}

function OllamaStep() {
  const status = useOllamaStatus();
  const pull = useOllamaPull();
  const [openFailed, setOpenFailed] = useState(false);
  const s = status.data;
  const percent = pull.totalMb > 0 ? Math.round((pull.completedMb / pull.totalMb) * 100) : 0;
  return (
    <>
      <Intro title="Text model" eyebrow="Qwen through Ollama">
        Summaries are written by Qwen, running in Ollama on this Mac. Install Ollama from ollama.com if you don't have it,
        then open it — this page checks every few seconds.
      </Intro>
      <div className={styles.rows}>
        <Row title="Ollama" hint={s?.running ? "Running." : openFailed ? "Not found in Applications — install it first." : "Not running."}>
          {s?.running ? (
            <span className="chip chipBrand">Running</span>
          ) : (
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openOllamaApp().then((ok) => setOpenFailed(!ok))}>
              Open Ollama
            </button>
          )}
        </Row>
        <Row
          title={s?.model ?? "Qwen"}
          hint={
            pull.running
              ? `${pull.status}${pull.totalMb ? ` · ${pull.completedMb} of ${pull.totalMb} MB (${percent} %)` : ""}`
              : "About 6.6 GB, downloaded once by Ollama."
          }
        >
          {s?.model_installed || pull.done ? (
            <span className="chip chipBrand">Installed</span>
          ) : (
            <button type="button" className="btn btnSecondary btnCompact" disabled={!s?.running || pull.running} onClick={() => pull.start()}>
              {pull.running ? "Downloading…" : "Download"}
            </button>
          )}
        </Row>
      </div>
      {pull.error && (
        <p className="formError" role="alert">
          {pull.error.message}
        </p>
      )}
    </>
  );
}

function WorkspaceStep() {
  const workspaces = useWorkspaces();
  const count = workspaces.data?.length ?? 0;
  return (
    <>
      <Intro title="Your first workspace" eyebrow="One per company">
        Each company you work with gets its own workspace: its own meetings, brand and minutes template.{" "}
        {count > 0 ? `You have ${count} — add more any time in Settings → Workspaces.` : "Create one to start."}
      </Intro>
      <WorkspacesPanel />
    </>
  );
}

function Row({ title, hint, children }: { title: string; hint: string; children: React.ReactNode }) {
  return (
    <div className={`card ${styles.row}`}>
      <div>
        <p className={styles.rowTitle}>{title}</p>
        <p className="fieldHint">{hint}</p>
      </div>
      <div className={styles.rowAction}>{children}</div>
    </div>
  );
}
