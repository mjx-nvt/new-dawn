import { useState } from "react";

// SEE (stage A): static form; stage B creates the workspace on "Create workspace".
export default function IdentityStep() {
  const [name, setName] = useState("VIFC Fintech Hub");
  const [slug, setSlug] = useState("vifc");

  return (
    <section className="section">
      <h2 className="sectionTitle">Identity</h2>
      <p className="sectionSubtitle">Creating the workspace makes its folder and database; the rest can be finished later</p>
      <div className="fields">
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Slug</span>
          <input className="input" value={slug} onChange={(e) => setSlug(e.target.value)} />
          <span className="fieldHint">Folder: workspaces/{slug || "…"}/</span>
        </label>
      </div>
    </section>
  );
}
