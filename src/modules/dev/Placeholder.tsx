/**
 * Placeholder root. Replaced in P1.1 by the shell (sidebar + toolbar + content root)
 * and TanStack Router. Until then this proves the window, the typeface and the tokens.
 */
export default function App() {
  return (
    <div className="content-root" style={{ padding: "var(--space-6)" }}>
      <p
        style={{
          fontSize: "var(--t-xs)",
          fontWeight: 700,
          letterSpacing: ".1em",
          textTransform: "uppercase",
          color: "var(--brand)",
          marginBottom: "var(--space-2)",
        }}
      >
        Scaffold · pre-P0
      </p>
      <h1 style={{ fontSize: "var(--t-2xl)", fontWeight: 700, lineHeight: 1.1 }}>New Dawn</h1>
      <p style={{ marginTop: "var(--space-3)", color: "var(--ink-2)", maxWidth: 520 }}>
        If this reads in Be Vietnam Pro and the window has rounded corners, the scaffold is working.
        Next: open this folder in Claude Code and run prompt P0 from <code>docs/playbook.md</code>.
      </p>
    </div>
  );
}
