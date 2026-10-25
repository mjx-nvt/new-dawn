import { Component, type ErrorInfo, type ReactNode } from "react";

/** The last line of defence: a rendering error shows what broke instead of an empty window. */
export default class CrashScreen extends Component<{ children: ReactNode }, { error: Error | null }> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("New Dawn crashed while rendering", error, info.componentStack);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    return (
      <main className="content-root" style={{ padding: "var(--space-8)" }}>
        <p className="eyebrow">Something went wrong</p>
        <h1 className="pageTitle">This screen couldn't be shown</h1>
        <p className="fieldHint" style={{ marginTop: "var(--space-3)", userSelect: "text" }}>
          {error.message}
        </p>
        <button type="button" className="btn btnPrimary" style={{ marginTop: "var(--space-5)" }} onClick={() => window.location.reload()}>
          Reload
        </button>
      </main>
    );
  }
}
