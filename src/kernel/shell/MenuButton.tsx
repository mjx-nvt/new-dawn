import { useEffect, useRef, useState, type ReactNode } from "react";

export interface MenuItem {
  label: string;
  onSelect: () => void;
  danger?: boolean;
  disabled?: boolean;
  /** A hairline above this item. */
  separator?: boolean;
}

/** A button that opens a glass popover of actions (the ⋯ menu). Focus moves into the menu,
 *  arrows move between items, Esc / a click outside / choosing an item closes it and focus
 *  returns to the button. */
export default function MenuButton({
  items,
  label = "More actions",
  className = "btn btnGhost btnCompact",
  children = "⋯",
}: {
  items: MenuItem[];
  label?: string;
  className?: string;
  children?: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);

  const close = () => {
    setOpen(false);
    trigger.current?.focus();
  };

  useEffect(() => {
    if (!open) return;
    list.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    const onPointerDown = (e: PointerEvent) => !root.current?.contains(e.target as Node) && setOpen(false);
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") return close();
      if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
      const buttons = [...(list.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = buttons[(at + (e.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length];
      next?.focus();
      e.preventDefault();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  return (
    <div ref={root} style={{ position: "relative" }}>
      <button ref={trigger} type="button" className={className} aria-label={label} aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)}>
        {children}
      </button>
      {open && (
        <div ref={list} className="glass menu" role="menu">
          {items.map((item) => (
            <div key={item.label} role="none">
              {item.separator && <div className="menuSeparator" role="separator" />}
              <button
                type="button"
                role="menuitem"
                className={item.danger ? "menuItem menuItemDanger" : "menuItem"}
                disabled={item.disabled}
                onClick={() => {
                  close();
                  item.onSelect();
                }}
              >
                {item.label}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
