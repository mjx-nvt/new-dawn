import type { ReactNode } from "react";
import styles from "./ContentRoot.module.css";

/** The reading surface over the glass. It paints the app's own palette: a workspace's brand
 *  never reaches the UI (D-033). */
export default function ContentRoot({ children }: { children: ReactNode }) {
  return <main className={`content-root ${styles.root}`}>{children}</main>;
}
