import type { SuggestionKeyDownProps, SuggestionProps } from "@tiptap/suggestion";
import styles from "./Editor.module.css";

/** A suggestion's list (`@` mentions, the `/` menu): a glass menu under the caret, arrows and
 *  Enter pick, Escape closes. Plain DOM, as TipTap's suggestion drives it imperatively. */
export function menuList<T>(text: (item: T) => string, empty: string) {
  return () => {
    let root: HTMLDivElement | null = null;
    let items: T[] = [];
    let selected = 0;
    let command: ((item: T) => void) | null = null;
    const paint = () => {
      if (!root) return;
      root.innerHTML = "";
      if (!items.length) {
        const none = document.createElement("p");
        none.className = "ghostText";
        none.textContent = empty;
        root.appendChild(none);
        return;
      }
      items.forEach((item, i) => {
        const button = document.createElement("button");
        button.type = "button";
        button.setAttribute("role", "menuitem");
        button.className = i === selected ? `menuItem ${styles.mentionItemSelected}` : "menuItem";
        button.textContent = text(item);
        button.addEventListener("mousedown", (e) => e.preventDefault());
        button.addEventListener("click", () => command?.(item));
        root?.appendChild(button);
      });
      root.querySelectorAll("button")[selected]?.scrollIntoView({ block: "nearest" });
    };
    const place = (clientRect: SuggestionProps["clientRect"]) => {
      const rect = clientRect?.();
      if (!root || !rect) return;
      root.style.left = `${rect.left}px`;
      root.style.top = `${rect.bottom + 4}px`;
    };
    return {
      onStart: (props: SuggestionProps) => {
        root = document.createElement("div");
        root.className = `glass menu ${styles.mentionMenu}`;
        root.setAttribute("role", "menu");
        document.body.appendChild(root);
        items = props.items as T[];
        selected = 0;
        command = (item) => props.command(item);
        paint();
        place(props.clientRect);
      },
      onUpdate: (props: SuggestionProps) => {
        items = props.items as T[];
        selected = Math.min(selected, Math.max(0, items.length - 1));
        command = (item) => props.command(item);
        paint();
        place(props.clientRect);
      },
      onKeyDown: (props: SuggestionKeyDownProps) => {
        if (props.event.key === "ArrowDown") {
          selected = items.length ? (selected + 1) % items.length : 0;
          paint();
          return true;
        }
        if (props.event.key === "ArrowUp") {
          selected = items.length ? (selected - 1 + items.length) % items.length : 0;
          paint();
          return true;
        }
        if (props.event.key === "Enter") {
          // No match: the Enter is the text's ("1 /2", a path).
          const item = items[selected];
          if (!item) return false;
          command?.(item);
          return true;
        }
        if (props.event.key === "Escape") {
          root?.remove();
          root = null;
          return true;
        }
        return false;
      },
      onExit: () => {
        root?.remove();
        root = null;
      },
    };
  };
}
