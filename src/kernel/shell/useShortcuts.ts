import { useNavigate } from "@tanstack/react-router";
import { useEffect, useRef } from "react";

/** `g` then a letter: Twenty's go-to keys, with New Dawn's own pages. */
const GO_CRM: Record<string, string> = { p: "people", c: "companies", o: "opportunities", n: "notes", t: "tasks" };

export const SHORTCUTS: { keys: string; what: string }[] = [
  { keys: "⌘K", what: "The command menu: search, create, go anywhere" },
  { keys: "/", what: "Search records" },
  { keys: "g p · g c · g o · g n · g t", what: "Go to People, Companies, Opportunities, Notes, Tasks" },
  { keys: "g m · g d", what: "Go to Meetings, Today" },
  { keys: "⌘↩", what: "In the command menu: run the first result" },
  { keys: "Esc", what: "Close a menu, a sheet or an edit" },
  { keys: "⌘[ · ⌘]", what: "Back, forward" },
  { keys: "⌃⌘S · ⌥⌘S", what: "Sidebar: icons only, hidden" },
  { keys: "⌘,", what: "Settings" },
  { keys: "⌘N · ⇧⌘H", what: "Tasks: a new task, show or hide Completed" },
];

function inField(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || !el.tagName) return false;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable;
}

/** Twenty's keys (D-064, 0.10.7): ⌘K opens the command menu, `/` the record search, `g` then
 *  a letter goes to an object or a page — the chord waits a second, and nothing fires while
 *  typing in a field or while a sheet is open. macOS-reserved keys are left alone. */
export function useShortcuts({ onCommandMenu, onSearch }: { onCommandMenu: () => void; onSearch: () => void }) {
  const navigate = useNavigate();
  const chord = useRef<number | null>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.isComposing) return;
      // Never a second sheet over an open one (the menu itself included).
      const sheetOpen = !!document.querySelector("dialog[open]");
      if (e.metaKey && !e.shiftKey && !e.altKey && !e.ctrlKey && e.key.toLowerCase() === "k") {
        if (sheetOpen) return;
        e.preventDefault();
        onCommandMenu();
        return;
      }
      const plain = !e.metaKey && !e.ctrlKey && !e.altKey;
      if (!plain || inField(e.target) || sheetOpen) return;
      if (e.key === "/") {
        e.preventDefault();
        onSearch();
        return;
      }
      if (chord.current) {
        window.clearTimeout(chord.current);
        chord.current = null;
        const key = e.key.toLowerCase();
        if (key === "m") void navigate({ to: "/meetings", search: {} });
        else if (key === "d") void navigate({ to: "/today" });
        else if (GO_CRM[key]) void navigate({ to: "/crm", search: { tab: GO_CRM[key] } });
        else return;
        e.preventDefault();
        return;
      }
      if (e.key === "g") {
        chord.current = window.setTimeout(() => {
          chord.current = null;
        }, 1000);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navigate, onCommandMenu, onSearch]);
}
