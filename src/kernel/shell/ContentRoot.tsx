import type { ReactNode } from "react";
import styles from "./ContentRoot.module.css";

/** The brand-owned reading surface. applyBrand() (P1.4) sets tenant tokens on this element. */
export default function ContentRoot({ children }: { children: ReactNode }) {
  return <main className={`content-root ${styles.root}`}>{children}</main>;
}
