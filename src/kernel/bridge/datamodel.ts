import { useMutation, useQueryClient } from "@tanstack/react-query";
import { commands, type FieldInput, type FieldPatch, type FieldType, type ObjectInput, type ObjectPatch, type RelationInput, type RelationInputKind } from "../../bindings";
import { unwrap } from "./unwrap";

export type { FieldInput, FieldPatch, ObjectInput, ObjectPatch, RelationInput, RelationInputKind };

/** The data model settings (D-060, 0.10.5): custom objects, fields and relations, made as real
 *  tables and columns by the engine. Every change refreshes what reads the metadata. */
function useRefresh(slug: string | undefined) {
  const queryClient = useQueryClient();
  return () => {
    for (const key of ["objects", "object", "views", "view", "layout", "records", "record", "search"]) void queryClient.invalidateQueries({ queryKey: [key, slug ?? ""] });
  };
}

export function useCreateObject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (input: ObjectInput) => unwrap(commands.createObject(slug ?? "", input)), onSuccess: refresh });
}

export function useUpdateObject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: ({ id, patch }: { id: string; patch: ObjectPatch }) => unwrap(commands.updateObject(slug ?? "", id, patch)), onSuccess: refresh });
}

export function useSetObjectActive(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: ({ id, active }: { id: string; active: boolean }) => unwrap(commands.setObjectActive(slug ?? "", id, active)), onSuccess: refresh });
}

export function useDeleteObject(slug: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteObject(slug ?? "", id)), onSuccess: refresh });
}

export function useCreateField(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (input: FieldInput) => unwrap(commands.createField(slug ?? "", object ?? "", input)), onSuccess: refresh });
}

export function useUpdateField(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: ({ id, patch }: { id: string; patch: FieldPatch }) => unwrap(commands.updateField(slug ?? "", object ?? "", id, patch)), onSuccess: refresh });
}

export function useSetFieldActive(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: ({ id, active }: { id: string; active: boolean }) => unwrap(commands.setFieldActive(slug ?? "", object ?? "", id, active)), onSuccess: refresh });
}

export function useDeleteField(slug: string | undefined, object: string | undefined) {
  const refresh = useRefresh(slug);
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteField(slug ?? "", object ?? "", id)), onSuccess: refresh });
}

/** The types the picker offers, as Twenty names and describes them. */
export const FIELD_TYPES: { type: FieldType; label: string; hint: string }[] = [
  { type: "TEXT", label: "Text", hint: "A single line of text" },
  { type: "LONG_TEXT", label: "Long text", hint: "Several lines of text" },
  { type: "RICH_TEXT", label: "Rich text", hint: "Formatted text" },
  { type: "NUMBER", label: "Number", hint: "An integer or a decimal" },
  { type: "BOOLEAN", label: "Boolean", hint: "A true / false checkbox" },
  { type: "DATE", label: "Date", hint: "A day" },
  { type: "DATE_TIME", label: "Date & time", hint: "A day and a time" },
  { type: "SELECT", label: "Select", hint: "One choice from a list" },
  { type: "MULTI_SELECT", label: "Multi-select", hint: "Several choices from a list" },
  { type: "CURRENCY", label: "Currency", hint: "An amount with its currency" },
  { type: "LINKS", label: "Links", hint: "URLs with labels" },
  { type: "DOMAIN", label: "Domain", hint: "A website domain" },
  { type: "EMAILS", label: "Emails", hint: "A primary and other addresses" },
  { type: "PHONES", label: "Phones", hint: "Numbers with a country code" },
  { type: "ADDRESS", label: "Address", hint: "Street, city, state, country, postcode" },
  { type: "RATING", label: "Rating", hint: "One to five stars" },
  { type: "RELATION", label: "Relation", hint: "Records of another object" },
  { type: "ARRAY", label: "Array", hint: "A list of text values" },
];

export function typeLabel(type: FieldType): string {
  return FIELD_TYPES.find((t) => t.type === type)?.label ?? type.replace(/_/g, " ").toLowerCase();
}

/** Twenty's option colours, on the Majoris palette (green, red, yellow and orange keep their meaning). */
export const COLORS = ["green", "turquoise", "sky", "blue", "purple", "pink", "red", "orange", "yellow", "gray"] as const;

/** Tabler icon names, as Twenty keeps them; shown as a letter mark until the sidebar lists objects. */
export const ICONS = ["IconUser", "IconBuildingSkyscraper", "IconTargetArrow", "IconNotes", "IconCheckbox", "IconBriefcase", "IconHeartHandshake", "IconRocket", "IconBox", "IconTag", "IconCalendarEvent", "IconFile"] as const;

export const CURRENCIES = ["VND", "USD", "EUR", "GBP", "SGD", "JPY", "AUD", "CNY", "KRW", "THB"] as const;

export const RELATION_KINDS: { kind: RelationInputKind; label: string; hint: string }[] = [
  { kind: "MANY_TO_ONE", label: "Many to one", hint: "Each record links to one of the other object" },
  { kind: "ONE_TO_MANY", label: "One to many", hint: "Each record links to many of the other object" },
  { kind: "MANY_TO_MANY", label: "Many to many", hint: "Through a junction object, made for you" },
];
