import { useGlassStatus } from "../../kernel/bridge/glass";
import styles from "./ShellMockup.module.css";

/** Static review page for the shell: typography scale, a field section, ghost cards. No data. */
export default function ShellMockup() {
  const { data: glass } = useGlassStatus();

  return (
    <div className={styles.page}>
      <p className={styles.eyebrow}>Group A · Sample</p>
      <h1 className={styles.pageTitle}>Partner call with Acme Capital</h1>
      <p className={styles.meta}>VIFC · Partner call · 24 Oct 2026</p>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Details</h2>
        <p className={styles.sectionSubtitle}>What we know about this meeting</p>
        <div className={styles.fields}>
          <div>
            <p className={styles.label}>Title</p>
            <p className={styles.value}>Partner call with Acme Capital</p>
          </div>
          <div>
            <p className={styles.label}>Language</p>
            <p className={styles.value}>English</p>
          </div>
          <div>
            <p className={styles.label}>Duration</p>
            <p className="ghostText">Not yet captured</p>
          </div>
          <div>
            <p className={styles.label}>Notes</p>
            <p className="ghostText">Not yet captured</p>
          </div>
          <div>
            <p className={styles.label}>Window glass</p>
            {glass ? <p className={styles.value}>{glass}</p> : <p className="ghostText">Not yet captured</p>}
          </div>
        </div>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>Action items</h2>
        <p className={styles.sectionSubtitle}>Owners and due dates from the summary</p>
        <div className={styles.cards}>
          <div className={styles.card}>
            <p className={styles.cardTitle}>Send term sheet draft</p>
            <p className={styles.body}>Owner: Tien · due 31 Oct</p>
          </div>
          <div className={`${styles.slot} ghostCard`} />
          <div className={`${styles.slot} ghostCard`} />
        </div>
      </section>
    </div>
  );
}
