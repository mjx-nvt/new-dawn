import { useAppInfo } from "../../kernel/bridge/app";
import styles from "./Settings.module.css";

export default function AboutTab() {
  const { data } = useAppInfo();

  return (
    <section className={styles.section}>
      <h2 className={styles.sectionTitle}>New Dawn</h2>
      <p className={styles.sectionSubtitle}>Everything stays on this Mac: speech and text AI run locally</p>
      <div className={styles.fields}>
        {[
          ["Version", data?.version],
          ["Bundle identifier", data?.identifier],
        ].map(([label, value]) => (
          <div key={label}>
            <p className={styles.label}>{label}</p>
            {value ? <p className={styles.value}>{value}</p> : <p className="ghostText">Not yet captured</p>}
          </div>
        ))}
      </div>
    </section>
  );
}
