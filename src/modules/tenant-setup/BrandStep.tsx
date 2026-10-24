import { useState, type CSSProperties } from "react";
import styles from "./BrandStep.module.css";

// SEE (stage A): mock proposal. Stage B fills it from a starter pack, stage C from the .docx, stage D from the PDF.
const PACKS = [
  { id: "vifc", name: "VIFC Fintech Hub", primary: "#0132AD", note: "Blue, Playfair Display headings" },
  { id: "default", name: "Default", primary: "#3B5BDB", note: "Neutral, Be Vietnam Pro throughout" },
];

const COLOURS = [
  { key: "primary", label: "Primary", value: "#0132AD", source: "Template theme" },
  { key: "secondary", label: "Secondary", value: "", source: "" },
  { key: "accent", label: "Accent", value: "", source: "" },
  { key: "text", label: "Text", value: "#0B1220", source: "Template styles" },
  { key: "canvas", label: "Canvas", value: "#FFFFFF", source: "Pack" },
  { key: "positive", label: "Positive", value: "#1F7A4D", source: "Pack" },
  { key: "warning", label: "Warning", value: "#9A6A00", source: "Pack" },
  { key: "negative", label: "Negative", value: "#B3261E", source: "Pack" },
];

const FILES = [{ name: "TEMPLATE_Bien_ban_cuoc_hop.docx", role: "Minutes template" }];

export default function BrandStep() {
  const [pack, setPack] = useState("vifc");
  const [primary, setPrimary] = useState("#0132AD");
  const [radius, setRadius] = useState("soft");
  const [buttons, setButtons] = useState("rounded");

  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Start from</h2>
        <p className="sectionSubtitle">A starter pack fills the form below; anything can be edited</p>
        <div className={styles.packs}>
          {PACKS.map((p) => (
            <button
              key={p.id}
              type="button"
              aria-pressed={pack === p.id}
              className={`card ${styles.pack}`}
              onClick={() => {
                setPack(p.id);
                setPrimary(p.primary);
              }}
            >
              <span className={styles.packSwatch} style={{ background: p.primary }} />
              <span>
                <span className={styles.packName}>{p.name}</span>
                <span className="fieldHint">{p.note}</span>
              </span>
            </button>
          ))}
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Import from files</h2>
        <p className="sectionSubtitle">The minutes template is read for fonts, colours, logo, slogan and section headings</p>
        <div className={styles.drop}>
          <p>Drop the minutes template (.docx), brand guideline (PDF), logos and fonts</p>
          <button type="button" className="btn btnSecondary btnCompact">Choose files…</button>
        </div>
        <ul className={styles.files}>
          {FILES.map((f) => (
            <li key={f.name} className={styles.file}>
              <span className={styles.fileName}>{f.name}</span>
              <span className="chip">{f.role}</span>
            </li>
          ))}
          <li className={`${styles.fileGhost} ghostCard`} />
        </ul>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Colours</h2>
        <p className="sectionSubtitle">Primary drives buttons, chips and eyebrows; the rest of the ramp is derived</p>
        <div className={styles.swatches}>
          {COLOURS.map((c) => {
            const value = c.key === "primary" ? primary : c.value;
            return (
              <label key={c.key} className={styles.swatch}>
                <span className="fieldLabel">{c.label}</span>
                <span className={styles.swatchRow}>
                  {value ? (
                    <input
                      type="color"
                      className={styles.colour}
                      value={value}
                      onChange={(e) => c.key === "primary" && setPrimary(e.target.value.toUpperCase())}
                    />
                  ) : (
                    <span className={`${styles.colour} ghostAvatar`} />
                  )}
                  <span className={value ? "fieldValue" : "ghostText"}>{value || "Not set"}</span>
                </span>
                {c.source ? <span className="chip">{c.source}</span> : <span className="ghostPill chip">No source</span>}
              </label>
            );
          })}
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Logo</h2>
        <p className="sectionSubtitle">Candidates come from the template header and dropped logo files</p>
        <div className="fields">
          {["Primary", "On dark"].map((label) => (
            <div key={label} className="field">
              <span className="fieldLabel">{label}</span>
              <div className={styles.logos}>
                {[0, 1, 2].map((i) => (
                  <span key={i} className={`${styles.logo} ghostAvatar`} />
                ))}
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Slogan</h2>
        <p className="sectionSubtitle">Document headers show English first, then Vietnamese, one line each</p>
        <div className="fields">
          <label className="field">
            <span className="fieldLabel">English</span>
            <input className="input" defaultValue="The convergence of global capital and Vietnamese fintech" />
          </label>
          <label className="field">
            <span className="fieldLabel">Vietnamese</span>
            <input className="input" defaultValue="Nơi hội tụ dòng vốn toàn cầu và fintech Việt Nam" />
          </label>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Document fonts</h2>
        <p className="sectionSubtitle">Used only in exported documents; the app itself always uses Be Vietnam Pro</p>
        <div className="fields">
          <label className="field">
            <span className="fieldLabel">Heading</span>
            <input className="input" defaultValue="Playfair Display" />
          </label>
          <label className="field">
            <span className="fieldLabel">Body</span>
            <input className="input" defaultValue="Be Vietnam Pro" />
          </label>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Shape</h2>
        <p className="sectionSubtitle">Corner radius and button style inside the content area</p>
        <div className="fields">
          <div className="field">
            <span className="fieldLabel">Corners</span>
            <div className="segmented">
              {["sharp", "soft", "round"].map((r) => (
                <button key={r} type="button" aria-pressed={radius === r} onClick={() => setRadius(r)}>
                  {r[0].toUpperCase() + r.slice(1)}
                </button>
              ))}
            </div>
          </div>
          <div className="field">
            <span className="fieldLabel">Buttons</span>
            <div className="segmented">
              {["square", "rounded", "pill"].map((b) => (
                <button key={b} type="button" aria-pressed={buttons === b} onClick={() => setButtons(b)}>
                  {b[0].toUpperCase() + b.slice(1)}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Preview</h2>
        <p className="sectionSubtitle">How the content area will look</p>
        <div
          className={`card ${styles.preview}`}
          style={
            {
              "--brand": primary,
              "--brand-100": `color-mix(in srgb, ${primary} 12%, white)`,
              "--brand-700": `color-mix(in srgb, ${primary} 80%, black)`,
              "--brand-text": primary,
              "--button-shape": buttons === "pill" ? "999px" : buttons === "square" ? "4px" : "var(--radius-control)",
            } as CSSProperties
          }
        >
          <p className="eyebrow">Group A · Partner call</p>
          <p className={styles.previewTitle}>Meeting with Acme Capital</p>
          <div className={styles.previewRow}>
            <button type="button" className="btn btnPrimary">Export minutes</button>
            <button type="button" className="btn btnSecondary">Copy as email</button>
            <span className="chip chipBrand">Partner call</span>
          </div>
        </div>
      </section>
    </>
  );
}
