import styles from "./ReviewStep.module.css";

// SEE (stage A): static summary of what Save will write.
const FILES = [
  "brand/tokens.json",
  "brand/logo/primary.svg",
  "brand/logo/on_dark.svg",
  "brand/templates/meeting-minutes/template.docx",
  "brand/templates/meeting-minutes/manifest.json",
  "brand/guideline/VIFC_Brand_Guideline.pdf",
];

export default function ReviewStep() {
  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Identity</h2>
        <p className="sectionSubtitle">Already created</p>
        <div className="fields">
          <div className="field">
            <span className="fieldLabel">Name</span>
            <span className="fieldValue">VIFC Fintech Hub</span>
          </div>
          <div className="field">
            <span className="fieldLabel">Folder</span>
            <span className="fieldValue">workspaces/vifc/</span>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Brand</h2>
        <p className="sectionSubtitle">Colours, logo and slogan as they will be saved</p>
        <div className={`card ${styles.brand}`}>
          <div className={styles.swatches}>
            {["#0132AD", "#0B1220", "#FFFFFF", "#1F7A4D", "#9A6A00", "#B3261E"].map((hex) => (
              <span key={hex} className={styles.swatch} style={{ background: hex }} title={hex} />
            ))}
          </div>
          <span className={`${styles.logo} ghostAvatar`} />
          <div className={styles.slogan}>
            <span className="fieldValue">The convergence of global capital and Vietnamese fintech</span>
            <span className="fieldHint">Nơi hội tụ dòng vốn toàn cầu và fintech Việt Nam</span>
          </div>
        </div>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Meeting types</h2>
        <p className="sectionSubtitle">The first one is the default</p>
        <ul className={styles.types}>
          {[
            ["Partner call", "EN", "6 sections", true],
            ["Internal sync", "VI", "4 sections", false],
            ["Delegation visit", "EN", "6 sections", false],
          ].map(([name, lang, sections, isDefault]) => (
            <li key={String(name)} className={styles.type}>
              <span className={styles.typeName}>{name}</span>
              <span className="chip">{lang}</span>
              <span className="fieldHint">{sections}</span>
              {isDefault && <span className="chip chipBrand">Default</span>}
            </li>
          ))}
        </ul>
      </section>

      <section className="section">
        <h2 className="sectionTitle">Files</h2>
        <p className="sectionSubtitle">Written into the workspace folder on Save</p>
        <ul className={styles.paths}>
          {FILES.map((f) => (
            <li key={f}>{f}</li>
          ))}
        </ul>
      </section>
    </>
  );
}
