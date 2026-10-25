import styles from "./DocumentPreview.module.css";

/** A miniature first page in the workspace's brand — what the pack is for. Always paper-white:
 *  documents don't follow the Mac's appearance. */
export default function DocumentPreview({
  name,
  primary,
  text,
  headingFont,
  bodyFont,
  slogan,
  logo,
}: {
  name: string;
  primary: string;
  text?: string;
  headingFont?: string;
  bodyFont?: string;
  slogan?: string;
  logo?: string | null;
}) {
  const heading = { color: primary, fontFamily: headingFont ? `'${headingFont}', var(--font)` : undefined };
  const body = { color: text || undefined, fontFamily: bodyFont ? `'${bodyFont}', var(--font)` : undefined };
  return (
    <div className={styles.page} style={body} aria-hidden="true">
      <div className={styles.head}>
        {logo ? (
          <img className={styles.logo} src={logo} alt="" />
        ) : (
          <span className={styles.mark} style={{ background: primary }}>
            {name.charAt(0)}
          </span>
        )}
        <span className={styles.eyebrow} style={{ color: primary }}>
          Meeting minutes
        </span>
      </div>
      <p className={styles.title} style={heading}>
        Quarterly partner review
      </p>
      <p className={styles.meta}>24 October 2026 · 45 minutes · 6 attendees</p>
      <div className={styles.rule} style={{ background: primary }} />
      <p className={styles.line}>The partners agreed the Q4 priorities and confirmed the fund timeline.</p>
      <p className={styles.line}>Decision — Series A terms approved; term sheet to be issued by Friday.</p>
      <p className={styles.footer}>{slogan || name}</p>
    </div>
  );
}
