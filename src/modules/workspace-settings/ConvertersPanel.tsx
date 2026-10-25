import { useConverters } from "../../kernel/bridge/minutes";
import styles from "./Settings.module.css";

/** Which apps on this Mac can turn the minutes into a PDF, in the order they are tried. */
export default function ConvertersPanel() {
  const converters = useConverters();

  return (
    <section className="section">
      <h2 className="sectionTitle">PDF converters</h2>
      <p className="sectionSubtitle">Tried in this order; the first that works makes the PDF</p>
      <div className={styles.fields}>
        {converters.data
          ? converters.data.map((c) => (
              <div key={c.converter}>
                <p className={styles.label}>{c.name}</p>
                <div className={styles.permission}>
                  <span className={c.path ? "chip chipPrimary" : "chip"}>{c.path ? "Found" : "Not installed"}</span>
                </div>
                {c.path ? <p className="fieldHint">{c.path}</p> : <p className="ghostText">No app found in Applications</p>}
              </div>
            ))
          : [0, 1, 2].map((i) => <div key={i} className={`${styles.slot} ghostCard`} />)}
      </div>
      {converters.error && (
        <p className="formError" role="alert">
          {converters.error.message}
        </p>
      )}
      <p className="fieldHint">
        Word and Pages are driven through Apple Events: macOS asks once whether New Dawn may control them. Exports are Word
        documents; a PDF is made only when you ask for one.
      </p>
    </section>
  );
}
