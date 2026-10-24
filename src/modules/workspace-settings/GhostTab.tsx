import styles from "./Settings.module.css";

interface GhostSection {
  title: string;
  subtitle: string;
}

/** A tab whose feature isn't built yet: the structure is drawn, the content is dashed ghosts. */
export default function GhostTab({ sections }: { sections: GhostSection[] }) {
  return (
    <>
      {sections.map((section) => (
        <section key={section.title} className={styles.section}>
          <h2 className={styles.sectionTitle}>{section.title}</h2>
          <p className={styles.sectionSubtitle}>{section.subtitle}</p>
          <div className={`${styles.slot} ghostCard`} />
        </section>
      ))}
    </>
  );
}
