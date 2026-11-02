import type { Project, ProjectStatus, UpdateField } from "../../kernel/bridge/projects";
import { formatDay as formatDate } from "../../kernel/format/format";

export const STATUSES: ProjectStatus[] = ["active", "on_hold", "done"];

export const STATUS_LABEL: Record<ProjectStatus, string> = { active: "Active", on_hold: "On hold", done: "Done" };

export const FIELD_LABEL: Record<UpdateField, string> = {
  milestone: "Milestone",
  phase: "Phase",
  status: "Status",
  risk: "New risk",
  decision: "Decision",
  note: "Note",
};

/** "15 Nov 2026" (or the owner's date format) for a YYYY-MM-DD calendar day (no time zone shift). */
export function formatDay(day: string): string {
  return formatDate(day);
}

/** "Audit visit · by 15 Nov 2026", or "" when there's no milestone. */
export function milestoneText(p: Pick<Project, "next_milestone" | "milestone_due">): string {
  if (!p.next_milestone) return "";
  return p.milestone_due ? `${p.next_milestone} · by ${formatDay(p.milestone_due)}` : p.next_milestone;
}
