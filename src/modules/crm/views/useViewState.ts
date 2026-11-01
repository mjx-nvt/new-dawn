import { useEffect, useState } from "react";
import type { FieldMeta, Json, ObjectDef, Query } from "../../../kernel/bridge/records";
import type { ViewDef } from "../../../kernel/bridge/views";

/** The working copy of a view (D-061): what the table shows right now — the saved view plus the
 *  owner's unsaved changes. "Update view" writes it back; "Reset" drops the changes. The shapes
 *  are the engine's inputs with every field present. */
export interface WorkingField {
  field_id: string;
  is_visible: boolean;
  position: number;
  size: number;
  aggregate_operation: string | null;
}

export interface Rule {
  field_id: string;
  /** A composite's part ('' = the whole field). */
  sub: string;
  operand: string;
  value: Json;
  /** The rule group it belongs to, or null for the view's own AND. */
  group_id: string | null;
  position: number;
}

export interface RuleGroup {
  id: string;
  parent_id: string | null;
  /** AND · OR · NOT */
  logical: string;
  position: number;
}

export interface WorkingSort {
  field_id: string;
  sub: string;
  /** ASC · DESC */
  direction: string;
  position: number;
}

export interface WorkingGroup {
  field_value: string;
  is_visible: boolean;
  position: number;
}

export interface Working {
  /** Every field of the view, visible or not, in order. */
  fields: WorkingField[];
  filters: { groups: RuleGroup[]; filters: Rule[] };
  sorts: WorkingSort[];
  /** The select field the table groups by (its id), or none. */
  groupBy: string | null;
  /** MANUAL · ASC · DESC */
  groupSort: string;
  groups: WorkingGroup[];
}

export function fromView(def: ViewDef): Working {
  return {
    fields: [...def.fields]
      .sort((a, b) => (a.position ?? 0) - (b.position ?? 0))
      .map((f) => ({ field_id: f.field_id, is_visible: f.is_visible, position: f.position ?? 0, size: f.size, aggregate_operation: f.aggregate_operation ?? null })),
    filters: {
      groups: def.filter_groups.map((g) => ({ id: g.id, parent_id: g.parent_id ?? null, logical: g.logical, position: g.position ?? 0 })),
      filters: def.filters.map((f) => ({ field_id: f.field_id, sub: f.sub, operand: f.operand, value: f.value, group_id: f.group_id ?? null, position: f.position ?? 0 })),
    },
    sorts: def.sorts.map((s) => ({ field_id: s.field_id, sub: s.sub, direction: s.direction, position: s.position ?? 0 })),
    groupBy: def.view.group_by_field_id ?? null,
    groupSort: def.view.group_sort,
    groups: def.groups.map((g) => ({ field_value: g.field_value, is_visible: g.is_visible, position: g.position ?? 0 })),
  };
}

export function same(a: Working, b: Working): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** The working copy follows the saved view until the owner changes it. */
export function useWorking(def: ViewDef | undefined): [Working | null, (next: Working) => void, () => void] {
  const [working, setWorking] = useState<Working | null>(null);
  const [from, setFrom] = useState<string | null>(null);
  const saved = def ? JSON.stringify(fromView(def)) : null;
  useEffect(() => {
    if (saved !== from) {
      setWorking(def ? fromView(def) : null);
      setFrom(saved);
    }
  }, [saved, from, def]);
  return [working, setWorking, () => def && setWorking(fromView(def))];
}

/** What a field's rules can do, by its type. */
export type Kind = "text" | "number" | "date" | "select" | "multi" | "boolean" | "relation";

export function kindOf(field: FieldMeta): Kind {
  switch (field.type) {
    case "NUMBER":
    case "RATING":
    case "CURRENCY":
    case "POSITION":
      return "number";
    case "DATE":
    case "DATE_TIME":
      return "date";
    case "SELECT":
      return "select";
    case "MULTI_SELECT":
      return "multi";
    case "BOOLEAN":
      return "boolean";
    case "RELATION":
      return "relation";
    default:
      return "text";
  }
}

export const OPS: Record<Kind, { op: string; label: string }[]> = {
  text: [
    { op: "contains", label: "Contains" },
    { op: "notContains", label: "Doesn't contain" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
  number: [
    { op: "is", label: "Is" },
    { op: "isNot", label: "Is not" },
    { op: "gte", label: "Greater than or equal" },
    { op: "lte", label: "Less than or equal" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
  date: [
    { op: "is", label: "Is" },
    { op: "isRelative", label: "Is relative" },
    { op: "isInPast", label: "Is in past" },
    { op: "isInFuture", label: "Is in future" },
    { op: "isToday", label: "Is today" },
    { op: "isBefore", label: "Is before" },
    { op: "isAfter", label: "Is after or equal" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
  select: [
    { op: "is", label: "Is" },
    { op: "isNot", label: "Is not" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
  multi: [
    { op: "contains", label: "Contains" },
    { op: "notContains", label: "Doesn't contain" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
  boolean: [{ op: "is", label: "Is" }],
  relation: [
    { op: "is", label: "Is" },
    { op: "isNot", label: "Is not" },
    { op: "isEmpty", label: "Is empty" },
    { op: "isNotEmpty", label: "Is not empty" },
  ],
};

export function opLabel(kind: Kind, op: string): string {
  return OPS[kind].find((o) => o.op === op)?.label ?? op;
}

export function needsValue(op: string): boolean {
  return !["isEmpty", "isNotEmpty", "isToday", "isInPast", "isInFuture"].includes(op);
}

/** A composite's parts a rule can target (text parts; a currency's amount). */
export function subsOf(field: FieldMeta): { key: string; label: string }[] {
  switch (field.type) {
    case "FULL_NAME":
      return [
        { key: "firstName", label: "First name" },
        { key: "lastName", label: "Last name" },
      ];
    case "EMAILS":
      return [{ key: "primaryEmail", label: "Primary email" }];
    case "PHONES":
      return [{ key: "primaryPhoneNumber", label: "Number" }];
    case "LINKS":
    case "DOMAIN":
      return [
        { key: "primaryLinkUrl", label: "URL" },
        { key: "primaryLinkLabel", label: "Label" },
      ];
    case "ADDRESS":
      return [
        { key: "addressStreet1", label: "Street" },
        { key: "addressCity", label: "City" },
        { key: "addressState", label: "State" },
        { key: "addressCountry", label: "Country" },
        { key: "addressPostcode", label: "Postcode" },
      ];
    case "CURRENCY":
      return [{ key: "amountMicros", label: "Amount" }];
    default:
      return [];
  }
}

/** A rule just added: the field's first part, its type's first operator, no value yet (a
 *  boolean rule starts at true). Left out of the query until it has the value it needs. */
export function newRule(field: FieldMeta, groupId: string | null, position: number): Rule {
  return { field_id: field.id, sub: subsOf(field)[0]?.key ?? "", operand: OPS[kindOf(field)][0].op, value: kindOf(field) === "boolean" ? true : null, group_id: groupId, position };
}

export function blank(value: Json): boolean {
  return value == null || (typeof value === "string" && value.trim() === "") || (Array.isArray(value) && value.length === 0);
}

/** The fields a rule, a sort or a column can use: active, with columns, not the position. */
export function usableFields(object: ObjectDef): FieldMeta[] {
  return object.fields.filter((f) => f.is_active && f.type !== "POSITION" && (f.type !== "RELATION" || f.options.relation?.kind === "MANY_TO_ONE") && (!f.is_system || f.name === "createdBy"));
}

/** A chosen relation in a rule's value: the id, with the label the owner saw. */
export interface Chosen {
  id: string;
  label: string;
}

export function chosenOf(value: Json): Chosen[] {
  return Array.isArray(value) ? value.flatMap((v) => (v && typeof v === "object" && "id" in v ? [{ id: String((v as Chosen).id), label: String((v as Chosen).label ?? "") }] : [])) : [];
}

/** The engine's query for the working copy, plus the page's search and the deleted switch. */
export function queryOf(object: ObjectDef, w: Working, extra: { search: string; deleted: boolean; limit?: number }): Query {
  const byId = new Map(object.fields.map((f) => [f.id, f]));
  const rule = (f: Rule): Json | null => {
    const field = byId.get(f.field_id);
    if (!field || (needsValue(f.operand) && blank(f.value))) return null;
    let value: Json = f.value;
    if (field.type === "RELATION") value = chosenOf(f.value).map((c) => c.id);
    if (field.type === "CURRENCY" && typeof value === "number" && f.sub === "amountMicros") value = Math.round(value * 1_000_000);
    return { field: field.name, sub: f.sub || null, op: f.operand, value };
  };
  const group = (id: string | null): { and: Json[] } => {
    const items: Json[] = w.filters.filters.filter((f) => (f.group_id ?? null) === id).map(rule).filter((x): x is Json => x !== null);
    for (const g of w.filters.groups.filter((g) => (g.parent_id ?? null) === id)) {
      const inner = group(g.id);
      if (!inner.and.length) continue;
      items.push(g.logical === "OR" ? { or: inner.and } : g.logical === "NOT" ? { not: inner } : inner);
    }
    return { and: items };
  };
  const filter = group(null);
  return {
    filter: filter.and.length ? filter : null,
    sort: w.sorts.flatMap((s) => {
      const field = byId.get(s.field_id);
      return field ? [{ field: field.name, sub: s.sub || null, desc: s.direction === "DESC" }] : [];
    }),
    search: extra.search || null,
    deleted: extra.deleted,
    limit: extra.limit ?? null,
    offset: null,
  };
}

/** A fresh id for a rule group made in the UI (the engine assigns the real one on save). */
export function tempId(): string {
  return `tmp-${Math.random().toString(36).slice(2, 10)}`;
}

export function nextPosition(items: { position: number }[]): number {
  return items.reduce((max, i) => Math.max(max, i.position), -1) + 1;
}
