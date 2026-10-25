import { useState } from "react";
import { useDeleteTerm, useSaveTerm, useVocabulary, useWhisperPrompt, type Term } from "../../kernel/bridge/vocabulary";
import DangerButton from "../../kernel/shell/DangerButton";
import { SparkleIcon } from "../../kernel/shell/icons";
import styles from "./VocabularyPanel.module.css";

/** "a, b ,, c" → ["a", "b", "c"] */
const split = (text: string) =>
  text
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean);

/** Workspace → Vocabulary (D-036): terms and how Whisper mishears them. Learned from your
 *  transcript corrections or added here; guides Whisper and corrects each new transcript. */
export default function VocabularyPanel({ slug }: { slug: string }) {
  const terms = useVocabulary(slug);
  const prompt = useWhisperPrompt(slug);
  const save = useSaveTerm(slug);
  const remove = useDeleteTerm(slug);
  const [term, setTerm] = useState("");
  const [heard, setHeard] = useState("");
  const error = terms.error ?? save.error ?? remove.error;

  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Vocabulary</h2>
        <p className="sectionSubtitle">Names and terms Whisper should spell right. Correct a line in a transcript and it's learned here.</p>

        <div className={`card ${styles.prompt}`}>
          <p className={`${styles.promptTitle} aiMark`}>
            <SparkleIcon />
            What Whisper is told before each transcription
          </p>
          {prompt.data ? <p className={styles.promptText}>{prompt.data}</p> : <p className="ghostText">Not yet captured</p>}
          <p className="fieldHint">People come from the People tab; their other spellings are corrected too.</p>
        </div>

        <form
          className={styles.add}
          onSubmit={(e) => {
            e.preventDefault();
            if (!term.trim()) return;
            save.mutate({ id: null, term, heard_as: split(heard) }, { onSuccess: () => (setTerm(""), setHeard("")) });
          }}
        >
          <label className="field">
            <span className="fieldLabel">Term</span>
            <input className="input" value={term} placeholder="WiredScore" onChange={(e) => setTerm(e.target.value)} />
          </label>
          <label className="field">
            <span className="fieldLabel">Misheard as (comma-separated)</span>
            <input className="input" value={heard} placeholder="Wired Score, Wire Score" onChange={(e) => setHeard(e.target.value)} />
          </label>
          <button type="submit" className="btn btnPrimary" disabled={!term.trim() || save.isPending}>
            Add
          </button>
        </form>
        {error && (
          <p className="formError" role="alert">
            {error.message}
          </p>
        )}
      </section>

      <section className="section">
        <h2 className="sectionTitle">Terms</h2>
        {terms.data?.length ? (
          <div className="group">
            {terms.data.map((t) => (
              <TermRow key={t.id} term={t} onSave={(heardAs) => save.mutate({ id: t.id, term: t.term, heard_as: heardAs })} onDelete={() => remove.mutate(t.id)} />
            ))}
          </div>
        ) : (
          <>
            <p className="ghostText">Not yet captured — correct a name or term in a transcript, or add one above.</p>
            <div className={`${styles.ghost} ghostCard`} />
            <div className={`${styles.ghost} ghostCard`} />
          </>
        )}
      </section>
    </>
  );
}

function TermRow({ term, onSave, onDelete }: { term: Term; onSave: (heardAs: string[]) => void; onDelete: () => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(term.heard_as.join(", "));
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">
          {term.term} <span className={term.source === "learned" ? "chip chipPrimary" : "chip"}>{term.source === "learned" ? "Learned" : "Added"}</span>
        </p>
        {editing ? (
          <input
            className="input"
            autoFocus
            aria-label={`Misheard spellings of ${term.term}`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                onSave(split(draft));
                setEditing(false);
              } else if (e.key === "Escape") setEditing(false);
            }}
          />
        ) : (
          <p className="rowHint">{term.heard_as.length ? `Misheard as ${term.heard_as.join(", ")}` : "Guides Whisper only"}</p>
        )}
      </div>
      <div className="rowControl">
        <button
          type="button"
          className="btn btnGhost btnCompact"
          onClick={() => {
            setDraft(term.heard_as.join(", ")); // fresh: spellings may have been learned since
            setEditing((v) => !v);
          }}
        >
          {editing ? "Cancel" : "Edit"}
        </button>
        <DangerButton confirm={`Remove ${term.term}?`} onConfirm={onDelete}>
          Remove
        </DangerButton>
      </div>
    </div>
  );
}
