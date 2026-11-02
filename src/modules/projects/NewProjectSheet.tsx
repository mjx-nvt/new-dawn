import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { useCreatePmProject, usePmProjects, type Features, type Network } from "../../kernel/bridge/pm";
import { usePeople } from "../../kernel/bridge/people";
import OnOff from "../workspace-settings/OnOff";
import { COVERS, EMOJI, FEATURES, coverClass, suggestIdentifier } from "./pm";
import styles from "./Projects.module.css";

/** Plane's New project (D-073): the cover, the logo, the name with its identifier (suggested
 *  until you change it), a description, the network and the lead — then the features. */
export default function NewProjectSheet({ slug, onClose }: { slug: string; onClose: () => void }) {
  const create = useCreatePmProject(slug);
  const people = usePeople(slug).data ?? [];
  const navigate = useNavigate();
  const dialog = useRef<HTMLDialogElement>(null);
  const [step, setStep] = useState<1 | 2>(1);
  const [name, setName] = useState("");
  const [identifier, setIdentifier] = useState<string | null>(null);
  const [description, setDescription] = useState("");
  const [emoji, setEmoji] = useState("");
  const [cover, setCover] = useState("slate");
  const [network, setNetwork] = useState<Network>("private");
  const [lead, setLead] = useState("me");
  const [features, setFeatures] = useState<Features>({ cycles: true, modules: true, views: true, pages: true, intake: false });
  const id = identifier ?? suggestIdentifier(name);
  // Identifiers are unique: a taken one is said here, before the features step.
  const taken = (usePmProjects(slug).data ?? []).find((p) => p.identifier.toUpperCase() === id.toUpperCase());
  const close = () => !create.isPending && onClose();
  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  const submit = () =>
    create.mutate(
      { name, identifier: id, description, emoji, cover, network, lead: lead || null, features },
      { onSuccess: (p) => void navigate({ to: "/projects/$projectId", params: { projectId: p.id }, search: {} }) },
    );

  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="new-project-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="new-project-title" className="sheetHeading">
          {step === 1 ? "New project" : `${name.trim() || "New project"} — features`}
        </h2>
        {step === 1 ? (
          <>
            <div className={`${styles.coverPick} ${coverClass(cover)}`}>
              <span className={styles.logo}>{emoji || id.slice(0, 2) || "?"}</span>
            </div>
            <div className="field">
              <span className="fieldLabel">Cover</span>
              <div className="segmented" role="group" aria-label="Cover">
                {COVERS.map((c) => (
                  <button key={c.key || "none"} type="button" aria-pressed={cover === c.key} onClick={() => setCover(c.key)}>
                    {c.label}
                  </button>
                ))}
              </div>
            </div>
            <div className="field">
              <span className="fieldLabel">Logo</span>
              <div className={styles.emojiRow}>
                <input className={`input ${styles.emojiInput}`} aria-label="Logo emoji" value={emoji} maxLength={8} placeholder="🙂" onChange={(e) => setEmoji(e.target.value)} />
                {EMOJI.map((e) => (
                  <button key={e} type="button" className="btn btnGhost btnCompact" aria-label={`Logo ${e}`} aria-pressed={emoji === e} onClick={() => setEmoji(e)}>
                    {e}
                  </button>
                ))}
              </div>
            </div>
            <div className="fields">
              <label className="field">
                <span className="fieldLabel">Name</span>
                <input className="input" value={name} autoFocus placeholder="VIFC Fintech Hub" onChange={(e) => setName(e.target.value)} />
              </label>
              <label className="field">
                <span className="fieldLabel">Identifier</span>
                <input className="input" value={id} maxLength={10} placeholder="VIFC" onChange={(e) => setIdentifier(e.target.value.toUpperCase().replace(/[^A-Z0-9]/g, ""))} />
              </label>
            </div>
            {taken && (
              <p className="formError" role="alert">
                {id} is {taken.name}’s identifier — identifiers are unique
              </p>
            )}
            <label className="field">
              <span className="fieldLabel">Description</span>
              <textarea className="input" rows={3} value={description} placeholder="What the project is for" onChange={(e) => setDescription(e.target.value)} />
            </label>
            <div className="fields">
              <div className="field">
                <span className="fieldLabel">Network</span>
                <div className="segmented" role="group" aria-label="Network">
                  <button type="button" aria-pressed={network === "private"} onClick={() => setNetwork("private")}>
                    Private
                  </button>
                  <button type="button" aria-pressed={network === "public"} onClick={() => setNetwork("public")}>
                    Public
                  </button>
                </div>
              </div>
              <label className="field">
                <span className="fieldLabel">Lead</span>
                <select className="input" value={lead} onChange={(e) => setLead(e.target.value)}>
                  <option value="">No lead</option>
                  <option value="me">You</option>
                  {people.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </>
        ) : (
          <div className="group">
            {FEATURES.map((f) => (
              <div key={f.key} className="groupRow">
                <div className="rowText">
                  <p className="rowLabel">{f.label}</p>
                  <p className="rowHint">{f.hint}</p>
                </div>
                <div className="rowControl">
                  <OnOff label={f.label} value={features[f.key]} onChange={(on) => setFeatures({ ...features, [f.key]: on })} />
                </div>
              </div>
            ))}
          </div>
        )}
        {create.error && (
          <p className="formError" role="alert">
            {create.error.message}
          </p>
        )}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={create.isPending} onClick={step === 1 ? onClose : () => setStep(1)}>
            {step === 1 ? "Cancel" : "Back"}
          </button>
          {step === 1 ? (
            <button type="button" className="btn btnPrimary" disabled={!name.trim() || !id || !!taken} onClick={() => setStep(2)}>
              Next
            </button>
          ) : (
            <button type="button" className="btn btnPrimary" disabled={create.isPending} onClick={submit}>
              {create.isPending ? "Creating…" : "Create project"}
            </button>
          )}
        </div>
      </div>
    </dialog>
  );
}
