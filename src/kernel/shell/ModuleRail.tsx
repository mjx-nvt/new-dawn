import { Link, type LinkProps } from "@tanstack/react-router";
import type { ReactNode } from "react";
import styles from "./ModuleRail.module.css";

/** The module rail (D-089): a module's navigation, inside the content panel. `compact` under
 *  1280 pt: icons and marks with tooltips. Rows are `RailLink`s; groups have a label and an
 *  optional control (✎). */
export default function ModuleRail({ label, compact, children }: { label: string; compact: boolean; children: ReactNode }) {
  return (
    <nav className={compact ? `${styles.rail} ${styles.compact}` : styles.rail} aria-label={label}>
      {children}
    </nav>
  );
}

export function RailGroup({ label, control, children }: { label?: string; control?: ReactNode; children: ReactNode }) {
  return (
    <div className={styles.group}>
      {label && (
        <div className={styles.groupHead}>
          <p className={styles.groupLabel}>{label}</p>
          {control}
        </div>
      )}
      {children}
    </div>
  );
}

/** A row: its icon or mark, the label (a tooltip when compact), a count at the end (a dot when
 *  compact). `part`: a step in under its parent. */
export function RailLink({
  icon,
  label,
  count = 0,
  compact,
  part = false,
  back = false,
  ...link
}: Omit<LinkProps, "children" | "className"> & { icon?: ReactNode; label: string; count?: number; compact: boolean; part?: boolean; back?: boolean }) {
  const classes = [styles.row, part && styles.part, back && styles.back].filter(Boolean).join(" ");
  return (
    <Link {...link} className={classes} activeProps={{ className: `${classes} ${styles.on}` }} title={compact ? (count ? `${label} · ${count}` : label) : undefined} aria-label={count ? `${label}, ${count}` : undefined}>
      {icon}
      <span className={compact ? "srOnly" : styles.label}>{label}</span>
      {count > 0 && (compact ? <span className={styles.dot} /> : <span className={styles.count}>{count}</span>)}
    </Link>
  );
}

/** A letter or emoji in a disc, for rows without an icon. */
export function RailMark({ children }: { children: ReactNode }) {
  return (
    <span className={styles.mark} aria-hidden>
      {children}
    </span>
  );
}

export { styles as railStyles };
