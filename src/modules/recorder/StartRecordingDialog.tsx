import { useEffect, useId, useRef, useState } from "react";
import type { LiveDirection } from "../../bindings";
import type { Language } from "../../kernel/bridge/meetings";
import { openPrivacySettings, useMicrophonePermission, useStartRecording } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace, useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { DIRECTION_LABEL, remember, remembered } from "../live/lines";
import MeetingFields, { FormatField, LanguageField } from "../meetings/MeetingFields";
import styles from "./StartRecordingDialog.module.css";

function defaultTitle(): string {
  const now = new Date().toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" });
  return `Meeting · ${now}`;
}

/** A live session's languages are its own: the meeting type's language mustn't replace them. */
const KEEP_LANGUAGE = () => {};

/** The live sheet's choices, remembered per Mac; the first of each is the default. */
const KEEP_KEY = "newdawn.liveKeep";
const DIRECTION_KEY = "newdawn.liveDirection";
const FORMAT_KEY = "newdawn.liveFormat";
const KEEPS = ["save", "quick"] as const;
const KEEP_LABEL: Record<(typeof KEEPS)[number], string> = { save: "Save as a meeting", quick: "Quick translation" };
const DIRECTIONS = ["vi_en", "en_vi", "both"] as const satisfies readonly LiveDirection[];
const FORMATS = ["discussion", "presentation"] as const;

/** ⌘R / ● Record: pick tenant, meeting type, title and language, then start capturing.
 *  `live` (the Live page): written down and translated as people speak (D-041) — saved as a
 *  meeting, or a quick translation that keeps nothing. */
export default function StartRecordingDialog({ live = false, onClose }: { live?: boolean; onClose: () => void }) {
  const { data: current } = useCurrentWorkspace();
  const [chosenSlug, setSlug] = useState("");
  const slug = chosenSlug || current?.slug || "";
  const [title, setTitle] = useState(defaultTitle);
  const [language, setLanguage] = useState<Language>(live ? "vi" : "en");
  const [keep, setKeep] = useState(() => remembered(KEEP_KEY, KEEPS));
  const [direction, setDirection] = useState(() => remembered(DIRECTION_KEY, DIRECTIONS));
  const [presentation, setPresentation] = useState(() => live && remembered(FORMAT_KEY, FORMATS) === "presentation");
  const [presenter, setPresenter] = useState("");
  const [typeId, setTypeId] = useState("");
  // Nothing is saved: no meeting, so no title or meeting type either.
  const quick = live && keep === "quick";
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
    if (live) {
      remember(KEEP_KEY, keep);
      remember(DIRECTION_KEY, direction);
      remember(FORMAT_KEY, presentation ? "presentation" : "discussion");
    }
    start.mutate(
      {
        slug,
        title: quick ? "" : title,
        language,
        meetingTypeId: quick ? null : typeId || null,
        presentation,
        presenter: presenter.trim() || null,
        live,
        save: !quick,
        direction,
      },
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
        <h2 id="record-title" className="sheetHeading">
          {live ? "Start live translation" : "New recording"}
        </h2>

        <MeetingFields
          slug={slug}
          onSlugChange={setSlug}
          typeId={typeId}
          onTypeChange={setTypeId}
          language={language}
          onLanguageChange={live ? KEEP_LANGUAGE : setLanguage}
          withType={!quick}
        />
        {live && (
          <>
            <Choice label="Keep" options={KEEPS} labels={KEEP_LABEL} value={keep} onChange={setKeep} />
            <Choice
              label="Languages"
              options={DIRECTIONS}
              labels={DIRECTION_LABEL}
              value={direction}
              onChange={setDirection}
              hint={direction === "both" ? "Each line's language is found as it's spoken, then translated into the other." : undefined}
            />
          </>
        )}
        <FormatField
          presentation={presentation}
          presenter={presenter}
          onChange={(p, name) => {
            setPresentation(p);
            setPresenter(name);
          }}
          hint={live ? (presentation ? "Longer lines, read as paragraphs." : "Your microphone and the call become separate lines — use headphones.") : undefined}
        />

        {!quick && (
          <label className="field">
            <span className="fieldLabel">Title</span>
            <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} />
          </label>
        )}

        {!live && <LanguageField id="record-language" value={language} onChange={setLanguage} />}

        {micBlocked ? (
          <div className={styles.notice}>
            <p>New Dawn can't use the microphone.</p>
            <button type="button" className="btn btnSecondary btnCompact" onClick={() => openPrivacySettings(false)}>
              Open Microphone settings
            </button>
          </div>
        ) : (
          <p className="fieldHint">
            {!live
              ? "Records your microphone and the call's audio from this Mac — no bot joins. Use headphones for a clean recording."
              : `Writes down what's said and translates each line as people speak — from your microphone and the call, all on this Mac. ${
                  quick ? "Nothing is recorded or saved." : "Saved as a meeting: the recording, its transcript and translation, and a summary."
                }`}
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
            disabled={!slug || (!quick && !title.trim()) || micBlocked || start.isPending}
            onClick={submit}
          >
            <span className="recordDot" />
            {start.isPending ? "Starting…" : live ? "Start live translation" : "Start recording"}
          </button>
        </div>
      </div>
    </dialog>
  );
}

/** One of a few options, as a segmented control, with a hint under it when there's one. */
function Choice<T extends string>({ label, options, labels, value, onChange, hint }: { label: string; options: readonly T[]; labels: Record<T, string>; value: T; onChange: (value: T) => void; hint?: string }) {
  const id = useId();
  return (
    <div className="field">
      <span className="fieldLabel" id={id}>
        {label}
      </span>
      <div className="segmented" role="group" aria-labelledby={id}>
        {options.map((option) => (
          <button key={option} type="button" aria-pressed={value === option} onClick={() => onChange(option)}>
            {labels[option]}
          </button>
        ))}
      </div>
      {hint && <p className="fieldHint">{hint}</p>}
    </div>
  );
}
