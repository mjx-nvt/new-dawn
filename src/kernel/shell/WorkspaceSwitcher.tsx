import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";
import { useCurrentWorkspace, useSetCurrentWorkspace, useWorkspaces } from "../bridge/workspaces";
import styles from "./WorkspaceSwitcher.module.css";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** The workspace badge at the top of the sidebar; click opens the switcher popover. */
export default function WorkspaceSwitcher({ open, onOpenChange }: Props) {
  const { data: current } = useCurrentWorkspace();
  const { data: workspaces } = useWorkspaces();
  const { mutate: choose } = useSetCurrentWorkspace();
  const navigate = useNavigate();
  const root = useRef<HTMLDivElement>(null);
  const active = (workspaces ?? []).filter((w) => !w.archived);

  useEffect(() => {
    if (!open) return;
    function onPointerDown(event: PointerEvent) {
      if (!root.current?.contains(event.target as Node)) onOpenChange(false);
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onOpenChange(false);
    }
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open, onOpenChange]);

  return (
    <div className={styles.root} ref={root}>
      <button
        type="button"
        className={styles.badge}
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
      >
        <span className={current ? styles.mark : `${styles.mark} ghostAvatar`}>
          {current ? current.name.charAt(0).toUpperCase() : ""}
        </span>
        <span className={current ? styles.badgeName : `${styles.badgeName} ghostText`}>
          {current ? current.name : "No workspace"}
        </span>
        <span className={styles.chevron} aria-hidden>⌄</span>
      </button>

      {open && (
        <div className={`glass ${styles.popover}`}>
          {active.length ? (
            active.map((workspace, index) => (
              <button
                key={workspace.id}
                type="button"
                aria-pressed={workspace.slug === current?.slug}
                className={styles.option}
                onClick={() => {
                  choose(workspace.slug);
                  onOpenChange(false);
                }}
              >
                <span className={styles.check}>{workspace.slug === current?.slug ? "✓" : ""}</span>
                <span className={styles.optionName}>{workspace.name}</span>
                {index < 9 && <span className={styles.hint}>⌘{index + 1}</span>}
              </button>
            ))
          ) : (
            <div className={`${styles.ghost} ghostCard`} />
          )}
          <div className={styles.divider} />
          <button
            type="button"
            className={styles.option}
            onClick={() => {
              onOpenChange(false);
              void navigate({ to: "/setup", search: { step: "identity" } });
            }}
          >
            <span className={styles.check} />
            <span className={styles.optionName}>New workspace…</span>
          </button>
        </div>
      )}
    </div>
  );
}
