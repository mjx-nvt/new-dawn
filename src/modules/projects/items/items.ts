import type { PmActivity, PmItem, PmRelationKind, Priority } from "../../../kernel/bridge/pmItems";
import { usePeople } from "../../../kernel/bridge/people";
import { whoName } from "../pm";
import type { PmLabel, PmState, StateGroup } from "../../../kernel/bridge/pm";

/** Plane's work items in New Dawn (D-074): the choices the list and the forms offer. */

export const PRIORITIES: { key: Priority; label: string }[] = [
  { key: "urgent", label: "Urgent" },
  { key: "high", label: "High" },
  { key: "medium", label: "Medium" },
  { key: "low", label: "Low" },
  { key: "none", label: "None" },
];

export const PRIORITY_RANK: Record<Priority, number> = { urgent: 0, high: 1, medium: 2, low: 3, none: 4 };

export const GROUP_LABEL: Record<StateGroup, string> = { backlog: "Backlog", unstarted: "Unstarted", started: "Started", completed: "Completed", cancelled: "Cancelled", triage: "Triage" };
const GROUP_RANK: Record<StateGroup, number> = { triage: 0, backlog: 1, unstarted: 2, started: 3, completed: 4, cancelled: 5 };

export type GroupBy = "state" | "group" | "priority" | "labels" | "assignees" | "created_by" | "none";
export type OrderBy = "manual" | "created" | "updated" | "start" | "due" | "priority";

export const GROUP_BY: { key: GroupBy; label: string }[] = [
  { key: "state", label: "State" },
  { key: "group", label: "State group" },
  { key: "priority", label: "Priority" },
  { key: "labels", label: "Labels" },
  { key: "assignees", label: "Assignees" },
  { key: "created_by", label: "Created by" },
  { key: "none", label: "None" },
];

export const ORDER_BY: { key: OrderBy; label: string }[] = [
  { key: "manual", label: "Manual" },
  { key: "created", label: "Last created" },
  { key: "updated", label: "Last updated" },
  { key: "start", label: "Start date" },
  { key: "due", label: "Due date" },
  { key: "priority", label: "Priority" },
];

/** What a row shows besides its title. */
export const PROPERTIES = ["id", "state", "priority", "assignees", "labels", "start", "due", "estimate", "sub_items"] as const;
export type Property = (typeof PROPERTIES)[number];
export const PROPERTY_LABEL: Record<Property, string> = { id: "ID", state: "State", priority: "Priority", assignees: "Assignees", labels: "Labels", start: "Start date", due: "Due date", estimate: "Estimate", sub_items: "Sub-work items" };

export interface Display {
  properties: Property[];
  subItems: boolean;
  emptyGroups: boolean;
}

export const DEFAULT_DISPLAY: Display = { properties: ["id", "state", "priority", "assignees", "labels", "due", "sub_items"], subItems: true, emptyGroups: true };

/** The display, remembered per project on this Mac (until views keep it, 0.11.4). */
export function loadDisplay(projectId: string): Display {
  try {
    const saved = localStorage.getItem(`newdawn.items.display.${projectId}`);
    return saved ? { ...DEFAULT_DISPLAY, ...(JSON.parse(saved) as Partial<Display>) } : DEFAULT_DISPLAY;
  } catch {
    return DEFAULT_DISPLAY;
  }
}

export function saveDisplay(projectId: string, display: Display) {
  try {
    localStorage.setItem(`newdawn.items.display.${projectId}`, JSON.stringify(display));
  } catch {
    // A private window: the display isn't remembered.
  }
}

/** The items in their order. */
export function ordered(items: PmItem[], order: OrderBy): PmItem[] {
  const by = (f: (i: PmItem) => string | number | null, desc = false) => (a: PmItem, b: PmItem) => {
    const [x, y] = [f(a), f(b)];
    if (x == null && y == null) return 0;
    if (x == null) return 1;
    if (y == null) return -1;
    const c = x < y ? -1 : x > y ? 1 : 0;
    return desc ? -c : c;
  };
  const sorted = [...items];
  switch (order) {
    case "created":
      return sorted.sort(by((i) => i.created_at, true));
    case "updated":
      return sorted.sort(by((i) => i.updated_at, true));
    case "start":
      return sorted.sort(by((i) => i.start_date));
    case "due":
      return sorted.sort(by((i) => i.target_date));
    case "priority":
      return sorted.sort(by((i) => PRIORITY_RANK[i.priority]));
    default:
      return sorted.sort(by((i) => i.sort_order ?? 0));
  }
}

export interface Group {
  key: string;
  label: string;
  /** What quick add gives an item made in this group. */
  value: { state_id?: string; priority?: Priority; labels?: string[]; assignees?: string[] };
  /** The state's colour (state groups). */
  color?: string;
  items: PmItem[];
}

/** The groups, every one the project has (the empty ones too), in their order. */
export function grouped(items: PmItem[], by: GroupBy, states: PmState[], labels: PmLabel[], who: (w: string) => string): Group[] {
  const visible = states.filter((s) => s.group !== "triage");
  switch (by) {
    case "state":
      return visible.map((s) => ({ key: s.id, label: s.name, color: s.color, value: { state_id: s.id }, items: items.filter((i) => i.state_id === s.id) }));
    case "group": {
      const groups = [...new Set(visible.map((s) => s.group))].sort((a, b) => GROUP_RANK[a] - GROUP_RANK[b]);
      return groups.map((g) => ({ key: g, label: GROUP_LABEL[g], value: { state_id: visible.find((s) => s.group === g)?.id }, items: items.filter((i) => i.state_group === g) }));
    }
    case "priority":
      return PRIORITIES.map((p) => ({ key: p.key, label: p.label, value: { priority: p.key }, items: items.filter((i) => i.priority === p.key) }));
    case "labels":
      return [
        // A label group can't be given to an item: only the labels themselves.
        ...labels.filter((l) => !labels.some((c) => c.parent_id === l.id)).map((l) => ({ key: l.id, label: l.name, color: l.color, value: { labels: [l.id] }, items: items.filter((i) => i.labels.includes(l.id)) })),
        { key: "none", label: "No label", value: {}, items: items.filter((i) => !i.labels.length) },
      ];
    case "assignees": {
      const all = [...new Set(items.flatMap((i) => i.assignees))];
      return [
        ...all.map((w) => ({ key: w, label: who(w), value: { assignees: [w] }, items: items.filter((i) => i.assignees.includes(w)) })),
        { key: "none", label: "No assignee", value: {}, items: items.filter((i) => !i.assignees.length) },
      ];
    }
    case "created_by": {
      const all = [...new Set(items.map((i) => i.created_by))];
      return all.map((w) => ({ key: w, label: who(w), value: {}, items: items.filter((i) => i.created_by === w) }));
    }
    default:
      return [{ key: "all", label: "All work items", value: {}, items }];
  }
}

/** Sub-items done, of those not cancelled: 0 … 1 (null when there are none). */
export function progress(item: PmItem): number | null {
  const counted = item.sub_items - item.sub_cancelled;
  return counted > 0 ? item.sub_done / counted : null;
}

/** "3 d 4 h", "2 h 5 min", "12 min", "under a minute". */
export function duration(ms: number): string {
  const min = Math.floor(ms / 60_000);
  if (min < 1) return "under a minute";
  const d = Math.floor(min / 1440);
  const h = Math.floor((min % 1440) / 60);
  const m = min % 60;
  if (d) return h ? `${d} d ${h} h` : `${d} d`;
  if (h) return m ? `${h} h ${m} min` : `${h} h`;
  return `${m} min`;
}

/** Time in each state, from the state changes (Transition): one stretch a stay, the last one
 *  still running. */
export function transitions(created: string, startState: string | null, acts: PmActivity[], now = Date.now()): { state: string; from: string; ms: number; current: boolean }[] {
  const moves = acts.filter((a) => a.field === "state");
  const first = acts.find((a) => a.kind === "created");
  let state = moves[0]?.old_value ?? startState ?? "";
  let from = first?.created_at ?? created;
  const out: { state: string; from: string; ms: number; current: boolean }[] = [];
  for (const m of moves) {
    out.push({ state: m.old_value || state, from, ms: Date.parse(m.created_at) - Date.parse(from), current: false });
    state = m.new_value;
    from = m.created_at;
  }
  out.push({ state, from, ms: now - Date.parse(from), current: true });
  return out;
}

/** An item as text to hand to an AI or a colleague (Plane's "Copy as prompt"). */
export function asPrompt(item: PmItem, labels: PmLabel[], who: (w: string) => string, subs: PmItem[]): string {
  const lines = [
    `${item.key} · ${item.name}`,
    `State: ${item.state_name} · Priority: ${PRIORITIES.find((p) => p.key === item.priority)?.label ?? item.priority}`,
    item.assignees.length ? `Assignees: ${item.assignees.map(who).join(", ")}` : "",
    item.labels.length ? `Labels: ${item.labels.map((l) => labels.find((x) => x.id === l)?.name ?? "").filter(Boolean).join(", ")}` : "",
    item.start_date || item.target_date ? `Dates: ${item.start_date ?? "—"} → ${item.target_date ?? "—"}` : "",
    item.estimate_value ? `Estimate: ${item.estimate_value}` : "",
    item.parent_key ? `Parent: ${item.parent_key} ${item.parent_name ?? ""}` : "",
    "",
    item.description_text || "(No description)",
    subs.length ? `\nSub-work items:\n${subs.map((s) => `- ${s.key} ${s.name} (${s.state_name})`).join("\n")}` : "",
  ];
  return lines.filter((l, i) => l !== "" || i === 7).join("\n").trim();
}

/** "You" or a person's name; an agent by its own name; the project automations as Automation. */
export function useWho(slug: string) {
  const people = usePeople(slug).data ?? [];
  return (w: string) => (w === "automation" ? "Automation" : w === "me" || w.length > 30 || people.some((p) => p.id === w) ? whoName(w, people) : w);
}

/** One activity row as a sentence ("You changed the state from Backlog to In Progress"). */
export function sentence(a: Pick<PmActivity, "kind" | "field" | "old_value" | "new_value" | "actor">, who: (w: string) => string): string {
  const actor = who(a.actor);
  switch (a.kind) {
    case "related":
      return `${actor} marked it ${a.new_value}`;
    case "unrelated":
      return `${actor} removed the relation ${a.old_value}`;
    case "linked":
      return `${actor} added the link ${a.new_value}`;
    case "unlinked":
      return `${actor} removed the link ${a.old_value}`;
    case "attached":
      return `${actor} attached ${a.new_value}`;
    case "detached":
      return `${actor} removed the attachment ${a.old_value}`;
    case "created":
      return `${actor} created the work item`;
    case "commented":
      return `${actor} commented`;
    case "archived":
      return `${actor} archived the work item`;
    case "restored":
      return `${actor} restored the work item`;
    case "copied":
      return `${actor} made it as a copy of ${a.new_value}`;
    default:
      if (a.field === "description") return `${actor} updated the description`;
      if (!a.new_value) return `${actor} removed the ${a.field}${a.old_value ? ` (${a.old_value})` : ""}`;
      if (!a.old_value) return `${actor} set the ${a.field} to ${a.new_value}`;
      return `${actor} changed the ${a.field} from ${a.old_value} to ${a.new_value}`;
  }
}

/** Plane's four relations, as the item reads them. */
export const RELATIONS: { kind: PmRelationKind; label: string }[] = [
  { kind: "relates_to", label: "Relates to" },
  { kind: "blocked_by", label: "Blocked by" },
  { kind: "blocking", label: "Blocking" },
  { kind: "duplicate", label: "Duplicate of" },
];
