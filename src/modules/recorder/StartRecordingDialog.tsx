import { useEffect, useRef, useState } from "react";
import type { Language } from "../../kernel/bridge/meetings";
import { openPrivacySettings, useMicrophonePermission, useStartRecording } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace, useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";
import MeetingFields, { LanguageField } from "../meetings/MeetingFields";
import styles from "./StartRecordingDialog.module.css";

function defaultTitle(): string {
  const now = new Date().toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
  return `Meeting · ${now}`;
}

/** ⌘R / ● Record: pick tenant, meeting type, title and language, then start capturing. */
export default function StartRecordingDialog({ onClose }: { onClose: () => void }) {
  const { data: current } = useCurrentWorkspace();
  const [chosenSlug, setSlug] = useState("");
  const slug = chosenSlug || current?.slug || "";
  const [title, setTitle] = useState(defaultTitle);
  const [language, setLanguage] = useState<Language>("en");
  const [typeId, setTypeId] = useState("");
  const mic = useMicrophonePermission();
  const start = useStartRecording();
  const switchWorkspace = useSetCurrentWorkspace();
  const dialog = useRef<HTMLDialogElement>(null);
  // While starting, the dialog can't be dismissed (the recording may already be running).
  const close = () => !start.isPending && onClose();

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  function submit() {
    start.mutate(
      { slug, title, language, meetingTypeId: typeId || null },
      {
        onSuccess: () => {
          onClose();
          // The meetings list follows the current workspace; Stop re-checks this too.
          if (slug !== current?.slug) switchWorkspace.mutate(slug);
        },
      },
    );
  }

  const micBlocked = mic.data === "denied" || mic.data === "restricted";

  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="record-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="record-title" className="sheetHeading">New recording</h2>

        <MeetingFields
          slug={slug}
          onSlugChange={setSlug}
          typeId={typeId}
          onTypeChange={setTypeId}
          language={language}
          onLanguageChange={setLanguage}
        />

        <label className="field">
          <span className="fieldLabel">Title</span>
          <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} />
        </label>

        <LanguageField id="record-language" value={language} onChange={setLanguage} />

        {micBlocked ? (
          <div className={styles.notice}>
            <p>New Dawn can't use the microphone.</p>
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => openPrivacySettings(false)}>
              Open Microphone settings
            </button>
          </div>
        ) : (
          <p className="fieldHint">
            Records your microphone and the call's audio from this Mac — no bot joins. Use headphones for a clean recording.
            {mic.data === "not_determined" && " macOS will ask for microphone access."}
          </p>
        )}

        {start.error && <p className="formError">{start.error.message}</p>}

        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={start.isPending} onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btnRecord"
            disabled={!slug || !title.trim() || micBlocked || start.isPending}
            onClick={submit}
          >
            <span className="recordDot" />
            {start.isPending ? "Starting…" : "Start recording"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
