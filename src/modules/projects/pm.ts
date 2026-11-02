import type { EstimateKind, Features, StateGroup } from "../../kernel/bridge/pm";
import styles from "./Projects.module.css";

/** Plane's projects in New Dawn (D-073): the choices the forms offer. */

/** A cover band from the app's palette (no stock images: nothing leaves this Mac; the status
 *  colours keep their meaning, so they aren't covers). */
export const COVERS: { key: string; label: string }[] = [
  { key: "", label: "None" },
  { key: "slate", label: "Slate" },
  { key: "orange", label: "Dawn" },
];

/** The cover band's class. */
export function coverClass(cover: string): string {
  return cover === "slate" ? styles.coverSlate : cover === "orange" ? styles.coverOrange : styles.coverNone;
}

export const EMOJI = ["🚀", "🏙", "📊", "💡", "🛠", "📦", "🎯", "🧭", "🌱", "🏗"];

export const FEATURES: { key: keyof Features; label: string; hint: string }[] = [
  { key: "cycles", label: "Cycles", hint: "Timebox work into short, repeating periods." },
  { key: "modules", label: "Modules", hint: "Group work items into larger pieces of work." },
  { key: "views", label: "Views", hint: "Save filters, orders and layouts to come back to." },
  { key: "pages", label: "Pages", hint: "Write and keep documents beside the work." },
  { key: "intake", label: "Intake", hint: "Take requests in to triage before they become work items." },
];

export const GROUPS: { key: StateGroup; label: string }[] = [
  { key: "backlog", label: "Backlog" },
  { key: "unstarted", label: "Unstarted" },
  { key: "started", label: "Started" },
  { key: "completed", label: "Completed" },
  { key: "cancelled", label: "Cancelled" },
];

export const ESTIMATE_PRESETS: { name: string; kind: EstimateKind; values: string[] }[] = [
  { name: "Linear", kind: "points", values: ["1", "2", "3", "4", "5", "6"] },
  { name: "Fibonacci", kind: "points", values: ["1", "2", "3", "5", "8", "13"] },
  { name: "Squares", kind: "points", values: ["1", "4", "9", "16", "25", "36"] },
  { name: "T-shirt sizes", kind: "categories", values: ["XS", "S", "M", "L", "XL", "XXL"] },
  { name: "Easy to hard", kind: "categories", values: ["Easy", "Medium", "Hard", "Very hard"] },
];

/** As the Rust side makes them (modules/pm identifier_from): the initials of two words or more,
 *  else the first five letters or digits; upper case, at most ten. */
export function suggestIdentifier(name: string): string {
  const plain = name.normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[đĐ]/g, "d");
  const words = plain.split(/[^A-Za-z0-9]+/).filter(Boolean).map((w) => w.toUpperCase());
  if (!words.length) return "";
  return words.length === 1 ? words[0].slice(0, 5) : words.map((w) => w[0]).join("").slice(0, 10);
}

/** "me" reads "You"; a person's id their name. */
export function whoName(who: string | null | undefined, people: { id: string; name: string }[]): string {
  if (!who) return "";
  return who === "me" ? "You" : (people.find((p) => p.id === who)?.name ?? "Someone no longer here");
}
