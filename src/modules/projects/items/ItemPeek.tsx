import { useEffect, useRef } from "react";
import { inField } from "../../../kernel/shell/useShortcuts";
import ItemView from "./ItemView";
import styles from "./Items.module.css";

export type PeekMode = "side" | "modal" | "full";

/** A work item over its list (D-074, Plane's peek): beside it (side), in a modal, or over the
 *  whole page; the mode and the item are URL state. */
export default function ItemPeek({ slug, itemId, mode, tab, onTab, onMode, onOpen, onClose }: { slug: string; itemId: string; mode: PeekMode; tab?: string; onTab: (tab: string | undefined) => void; onMode: (m: PeekMode) => void; onOpen: (id: string) => void; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    if (mode === "modal") dialog.current?.showModal();
  }, [mode]);
  useEffect(() => {
    // Escape inside a field (a title, the @ list, a comment) is the field's, not the peek's.
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && mode !== "modal" && !e.defaultPrevented && !inField(e.target) && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, onClose]);
  const bar = (
    <div className={styles.peekBar}>
      <div className="segmented" role="group" aria-label="Peek">
        {(["side", "modal", "full"] as PeekMode[]).map((m) => (
          <button key={m} type="button" aria-pressed={m === mode} onClick={() => onMode(m)}>
            {m === "side" ? "Side" : m === "modal" ? "Modal" : "Full screen"}
          </button>
        ))}
      </div>
      <button type="button" className="btn btnGhost btnCompact" aria-label="Close" onClick={onClose}>
        ✕
      </button>
    </div>
  );
  const body = <ItemView key={itemId} slug={slug} itemId={itemId} tab={tab} onTab={onTab} onOpen={onOpen} onGone={onClose} />;
  if (mode === "modal") {
    return (
      <dialog
        ref={dialog}
        className={`glass sheet ${styles.peekModal}`}
        aria-label="Work item"
        onCancel={(e) => {
          e.preventDefault();
          onClose();
        }}
        onPointerDown={(e) => e.target === e.currentTarget && onClose()}
      >
        {bar}
        {body}
      </dialog>
    );
  }
  return (
    <aside className={`glass ${mode === "full" ? styles.peekFull : styles.peekSide}`} aria-label="Work item">
      {bar}
      {body}
    </aside>
  );
}
