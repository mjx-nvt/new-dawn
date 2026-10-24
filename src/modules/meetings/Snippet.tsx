import styles from "./Snippet.module.css";

/** Renders an FTS5 snippet whose matches are wrapped in U+0002 … U+0003 — as text, never HTML. */
export default function Snippet({ text }: { text: string }) {
  const parts = text.split(/(\u0002[^\u0003]*\u0003)/);
  return (
    <p className={styles.snippet}>
      {parts.map((part, i) =>
        part.startsWith("\u0002") ? (
          <mark key={i} className={styles.match}>
            {part.slice(1, -1)}
          </mark>
        ) : (
          part
        ),
      )}
    </p>
  );
}
