import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { pickAudioFile, useImportAudio, type Language } from "../../kernel/bridge/meetings";
import { useCurrentWorkspace, useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";
import styles from "./ImportAudioSheet.module.css";
import MeetingFields, { LanguageField } from "./MeetingFields";

/** File → Import Audio…: pick a file, tenant, type, title and language; creates the meeting. */
export default function ImportAudioSheet({ onClose }: { onClose: () => void }) {
  const { data: current } = useCurrentWorkspace();
  const [chosenSlug, setSlug] = useState("");
  const slug = chosenSlug || current?.slug || ""; // current may load after the sheet opens
  const [path, setPath] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [language, setLanguage] = useState<Language>("en");
  const [typeId, setTypeId] = useState("");
  const importAudio = useImportAudio();
  const switchWorkspace = useSetCurrentWorkspace();
  const navigate = useNavigate();
  const dialog = useRef<HTMLDialogElement>(null);
  const close = () => !importAudio.isPending && onClose();

  // Native modal: top layer, inert background, focus trap, Esc → `cancel` event.
  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  async function choose() {
    const picked = await pickAudioFile();
    if (!picked) return;
    setPath(picked);
    if (!title) setTitle(picked.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "");
  }

  function submit() {
    if (!path) return;
    importAudio.mutate(
      { slug, path, title, language, meetingTypeId: typeId || null },
      {
        onSuccess: async (meeting) => {
          // The meeting page reads the current workspace, so follow the import there.
          if (slug !== current?.slug) await switchWorkspace.mutateAsync(slug).catch(() => undefined);
          void navigate({ to: "/meetings/$meetingId", params: { meetingId: meeting.id }, search: { tab: "transcript" } });
        },
      },
    );
  }

  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="import-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="import-title" className="sheetHeading">Import audio</h2>

        <div className={styles.file}>
          {path ? <span className="fieldValue">{path.split("/").pop()}</span> : <span className="ghostText">No file chosen</span>}
          <button type="button" className="btn btnSecondary btnCompact" onClick={() => void choose()}>
            {path ? "Change…" : "Choose file…"}
          </button>
        </div>

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
          <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Partner call with Acme Capital" />
        </label>

        <LanguageField id="import-language" value={language} onChange={setLanguage} />

        {importAudio.error && <p className="formError">{importAudio.error.message}</p>}

        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={importAudio.isPending} onClick={onClose}>
            Cancel
          </button>
          <button
            type="button"
            className="btn btnPrimary"
            disabled={!path || !title.trim() || !slug || importAudio.isPending}
            onClick={submit}
          >
            {importAudio.isPending ? "Importing…" : "Import and transcribe"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
