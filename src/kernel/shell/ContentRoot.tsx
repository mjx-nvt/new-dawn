import type { ReactNode } from "react";
import styles from "./ContentRoot.module.css";

/** The reading surface over the glass. It paints the app's own palette: a workspace's brand
 *  never reaches the UI (D-033). With a module `rail` (D-089) the panel is two columns — the
 *  rail, sticky at the left, and the page. */
export default function ContentRoot({ rail, children }: { rail?: ReactNode; children: ReactNode }) {
  if (!rail) return <main className={`content-root ${styles.root}`}>{children}</main>;
  return (
    <main className={`content-root ${styles.root} ${styles.withRail}`}>
      {rail}
      <div className={`${styles.body} content-body`}>{children}</div>
    </main>
  );
}
