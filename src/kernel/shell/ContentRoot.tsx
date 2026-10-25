import type { ReactNode } from "react";
import { useCurrentWorkspace } from "../bridge/workspaces";
import { useBrandProps } from "../theme/applyBrand";
import styles from "./ContentRoot.module.css";

/** The brand-owned reading surface: the current workspace's tokens are set here as CSS
 *  variables, so everything inside follows its brand (the shell around it never does). */
export default function ContentRoot({ children }: { children: ReactNode }) {
  const { data: workspace } = useCurrentWorkspace();
  const brand = useBrandProps(workspace?.slug);
  return (
    <main className={`content-root ${styles.root}`} {...brand}>
      {children}
    </main>
  );
}
