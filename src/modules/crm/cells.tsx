import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { asRecord, selectOptions, useRecords, type FieldMeta, type RecordJson } from "../../kernel/bridge/records";
import { richExcerpt } from "../../kernel/editor/plain";
import { formatDateTime, formatNumber as formatNumeral } from "../../kernel/format/format";
import { formatDay } from "../projects/format";
import styles from "./Crm.module.css";

/** How a value of each Twenty field type reads and is edited in a table cell (D-060). The value
 *  is `unknown` from the engine; each type narrows it. */

const str = (v: unknown): string => (typeof v === "string" ? v : v == null ? "" : typeof v === "number" || typeof v === "boolean" ? String(v) : "");
const num = (v: unknown): number | null => (typeof v === "number" && Number.isFinite(v) ? v : null);
const list = (v: unknown): string[] => (Array.isArray(v) ? v.map((x) => str(x)).filter((x) => x) : []);

/** The colour class of a select option: Twenty's names on the Majoris palette — green, red,
 *  yellow and orange keep their meaning; the rest read as neutral chips. */
export function chipClass(color: string): string {
  switch (color) {
    case "green":
      return styles.chipGreen;
    case "red":
      return styles.chipRed;
    case "yellow":
      return styles.chipYellow;
    case "orange":
      return styles.chipOrange;
    default:
      return styles.chipNeutral;
  }
}

function formatNumber(n: number, decimals: number | null | undefined): string {
  return formatNumeral(n, decimals ?? 2);
}

/** The cell at rest. */
export function CellView({ field, record }: { field: FieldMeta; record: RecordJson }) {
  const value = record[field.name];
  switch (field.type) {
    case "BOOLEAN":
      return <span className={styles.muted}>{value === true ? "Yes" : "No"}</span>;
    case "NUMBER": {
      const n = num(value);
      return n == null ? null : <span className={styles.num}>{formatNumber(n, field.options.decimals)}</span>;
    }
    case "RATING": {
      const n = num(value) ?? 0;
      return n ? <span className={styles.rating}>{"★".repeat(n)}</span> : null;
    }
    case "DATE":
      return str(value) ? <span>{formatDay(str(value))}</span> : null;
    case "DATE_TIME":
      return str(value) ? <span>{formatDateTime(str(value))}</span> : null;
    case "SELECT": {
      const option = selectOptions(field).find((o) => o.value === str(value));
      return option ? <span className={`${styles.chip} ${chipClass(option.color)}`}>{option.label}</span> : null;
    }
    case "MULTI_SELECT": {
      const chosen = list(value);
      return (
        <span className={styles.chips}>
          {chosen.map((v) => {
            const option = selectOptions(field).find((o) => o.value === v);
            return (
              <span key={v} className={`${styles.chip} ${chipClass(option?.color ?? "")}`}>
                {option?.label ?? v}
              </span>
            );
          })}
        </span>
      );
    }
    case "CURRENCY": {
      const v = asRecord(value);
      const micros = num(v.amountMicros);
      if (micros == null) return null;
      return (
        <span className={styles.num}>
          {formatNumber(micros / 1_000_000, field.options.decimals ?? 2)} {str(v.currencyCode)}
        </span>
      );
    }
    case "LINKS":
    case "DOMAIN": {
      const v = asRecord(value);
      const url = str(v.primaryLinkUrl);
      return url ? <span className={styles.link}>{str(v.primaryLinkLabel) || url.replace(/^https?:\/\//, "")}</span> : null;
    }
    case "EMAILS": {
      const v = asRecord(value);
      const more = list(v.additionalEmails).length;
      return str(v.primaryEmail) ? (
        <span>
          {str(v.primaryEmail)}
          {more > 0 && <span className={styles.muted}> +{more}</span>}
        </span>
      ) : null;
    }
    case "PHONES": {
      const v = asRecord(value);
      return str(v.primaryPhoneNumber) ? <span>{[str(v.primaryPhoneCallingCode), str(v.primaryPhoneNumber)].filter(Boolean).join(" ")}</span> : null;
    }
    case "ADDRESS": {
      const v = asRecord(value);
      const line = [str(v.addressStreet1), str(v.addressCity), str(v.addressCountry)].filter(Boolean).join(", ");
      return line ? <span>{line}</span> : null;
    }
    case "FULL_NAME": {
      const v = asRecord(value);
      const name = `${str(v.firstName)} ${str(v.lastName)}`.trim();
      return name ? <span className={styles.title}>{name}</span> : null;
    }
    case "RELATION": {
      const v = asRecord(value);
      return str(v.label) ? <span className={`${styles.chip} ${styles.chipNeutral}`}>{str(v.label)}</span> : null;
    }
    case "ACTOR": {
      const v = asRecord(value);
      return <span className={styles.muted}>{str(v.name) || str(v.source).toLowerCase()}</span>;
    }
    case "ARRAY":
      return (
        <span className={styles.chips}>
          {list(value).map((v) => (
            <span key={v} className={`${styles.chip} ${styles.chipNeutral}`}>
              {v}
            </span>
          ))}
        </span>
      );
    case "JSON":
      return value == null ? null : <span className={styles.muted}>{JSON.stringify(value).slice(0, 60)}</span>;
    case "RICH_TEXT": {
      const excerpt = richExcerpt(value);
      return excerpt ? <span className={styles.text}>{excerpt}</span> : null;
    }
    default:
      return str(value) ? <span className={styles.text}>{str(value)}</span> : null;
  }
}

/** Types whose cell saves straight from the view (a tick), and types never edited in a table. */
export function editsInPlace(field: FieldMeta): boolean {
  return field.type === "BOOLEAN";
}

/** Never edited in a table: the engine's own fields, and a rich text (its editor is on the page). */
export function readOnly(field: FieldMeta): boolean {
  return field.type === "ACTOR" || field.type === "UUID" || field.type === "POSITION" || field.type === "RICH_TEXT";
}

interface EditorProps {
  slug: string;
  field: FieldMeta;
  record: RecordJson;
  onSave: (value: unknown) => void;
  onCancel: () => void;
}

function keys(onDone: () => void, onCancel: () => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter" && !e.nativeEvent.isComposing && !(e.currentTarget instanceof HTMLTextAreaElement)) onDone();
  };
}

/** The cell being edited: an input for the type; Enter or leaving saves, Escape cancels. */
export function CellEditor({ slug, field, record, onSave, onCancel }: EditorProps) {
  const value = record[field.name];
  switch (field.type) {
    case "TEXT":
    case "UUID":
      return <TextEditor initial={str(value)} onSave={onSave} onCancel={onCancel} />;
    case "LONG_TEXT":
      return <TextEditor initial={str(value)} multiline onSave={onSave} onCancel={onCancel} />;
    case "NUMBER":
      return <TextEditor initial={num(value)?.toString() ?? ""} kind="number" onSave={(v) => onSave(v === "" ? null : Number(v))} onCancel={onCancel} />;
    case "DATE":
      return <TextEditor initial={str(value)} kind="date" onSave={(v) => onSave(v === "" ? null : v)} onCancel={onCancel} />;
    case "DATE_TIME":
      return <TextEditor initial={str(value) ? toLocalInput(str(value)) : ""} kind="datetime-local" onSave={(v) => onSave(v === "" ? null : new Date(v).toISOString())} onCancel={onCancel} />;
    case "SELECT":
      return (
        <select className="input" autoFocus aria-label={field.label} defaultValue={str(value)} onChange={(e) => onSave(e.target.value || null)} onBlur={onCancel} onKeyDown={keys(onCancel, onCancel)}>
          <option value="">—</option>
          {selectOptions(field).map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      );
    case "MULTI_SELECT":
      return <MultiSelectEditor field={field} initial={list(value)} onSave={onSave} onCancel={onCancel} />;
    case "CURRENCY":
      return <CurrencyEditor initial={asRecord(value)} code={field.options.currency ?? ""} onSave={onSave} onCancel={onCancel} />;
    case "LINKS":
    case "DOMAIN":
      return <PartsEditor parts={[["primaryLinkUrl", "URL"], ["primaryLinkLabel", "Label"]]} initial={asRecord(value)} onSave={onSave} onCancel={onCancel} />;
    case "EMAILS":
      return <PartsEditor parts={[["primaryEmail", "Email"]]} initial={asRecord(value)} onSave={onSave} onCancel={onCancel} />;
    case "PHONES": {
      // A number saved with neither a calling code nor a country takes the field's default
      // country (Data model's setting); a blank number stays blank.
      const withCountry = (v: unknown) => {
        const p = asRecord(v);
        const fill = field.options.country && str(p.primaryPhoneNumber) && !str(p.primaryPhoneCallingCode) && !str(p.primaryPhoneCountryCode);
        onSave(fill ? { ...p, primaryPhoneCountryCode: field.options.country } : p);
      };
      return <PartsEditor parts={[["primaryPhoneCallingCode", "+84"], ["primaryPhoneNumber", "Number"]]} initial={asRecord(value)} onSave={withCountry} onCancel={onCancel} />;
    }
    case "ADDRESS":
      return (
        <PartsEditor
          parts={[["addressStreet1", "Street"], ["addressCity", "City"], ["addressState", "State"], ["addressCountry", "Country"], ["addressPostcode", "Postcode"]]}
          initial={asRecord(value)}
          onSave={onSave}
          onCancel={onCancel}
        />
      );
    case "FULL_NAME":
      return <PartsEditor parts={[["firstName", "First name"], ["lastName", "Last name"]]} initial={asRecord(value)} onSave={onSave} onCancel={onCancel} />;
    case "RATING":
      return <RatingEditor initial={num(value) ?? 0} onSave={onSave} onCancel={onCancel} />;
    case "RELATION":
      return <RelationEditor slug={slug} field={field} onSave={onSave} onCancel={onCancel} />;
    case "ARRAY":
      return <TextEditor initial={list(value).join(", ")} onSave={(v) => onSave(v.split(",").map((s) => s.trim()).filter(Boolean))} onCancel={onCancel} />;
    case "JSON":
      return (
        <TextEditor
          initial={value == null ? "" : JSON.stringify(value, null, 2)}
          multiline
          onSave={(v) => {
            try {
              onSave(v.trim() === "" ? null : (JSON.parse(v) as unknown));
            } catch {
              onCancel();
            }
          }}
          onCancel={onCancel}
        />
      );
    default:
      return null;
  }
}

function toLocalInput(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${at.getFullYear()}-${pad(at.getMonth() + 1)}-${pad(at.getDate())}T${pad(at.getHours())}:${pad(at.getMinutes())}`;
}

function TextEditor({ initial, kind = "text", multiline, onSave, onCancel }: { initial: string; kind?: string; multiline?: boolean; onSave: (v: string) => void; onCancel: () => void }) {
  const [draft, setDraft] = useState(initial);
  const done = () => (draft === initial ? onCancel() : onSave(draft));
  if (multiline) {
    return <textarea className={`input ${styles.editor} ${styles.area}`} autoFocus value={draft} rows={3} onChange={(e) => setDraft(e.target.value)} onBlur={done} onKeyDown={keys(done, onCancel)} />;
  }
  return <input className={`input ${styles.editor}`} type={kind} step={kind === "number" ? "any" : undefined} autoFocus value={draft} onChange={(e) => setDraft(e.target.value)} onBlur={done} onKeyDown={keys(done, onCancel)} />;
}

/** A composite's parts as small inputs in a row; leaving the last one, or Enter, saves them all. */
function PartsEditor({ parts, initial, onSave, onCancel }: { parts: [string, string][]; initial: RecordJson; onSave: (v: unknown) => void; onCancel: () => void }) {
  const [draft, setDraft] = useState<Record<string, string>>(Object.fromEntries(parts.map(([key]) => [key, str(initial[key])])));
  const changed = parts.some(([key]) => draft[key] !== str(initial[key]));
  const done = () => (changed ? onSave({ ...initial, ...draft }) : onCancel());
  return (
    <span className={styles.parts} onBlur={(e) => !e.currentTarget.contains(e.relatedTarget) && done()}>
      {parts.map(([key, placeholder], i) => (
        <input key={key} className={`input ${styles.editor}`} autoFocus={i === 0} placeholder={placeholder} aria-label={placeholder} value={draft[key] ?? ""} onChange={(e) => setDraft({ ...draft, [key]: e.target.value })} onKeyDown={keys(done, onCancel)} />
      ))}
    </span>
  );
}

function CurrencyEditor({ initial, code, onSave, onCancel }: { initial: RecordJson; code: string; onSave: (v: unknown) => void; onCancel: () => void }) {
  const micros = num(initial.amountMicros);
  const [amount, setAmount] = useState(micros == null ? "" : String(micros / 1_000_000));
  const [currency, setCurrency] = useState(str(initial.currencyCode) || code);
  const done = () => {
    const n = amount.trim() === "" ? null : Number(amount);
    if (n != null && !Number.isFinite(n)) return onCancel();
    onSave({ amountMicros: n == null ? null : Math.round(n * 1_000_000), currencyCode: currency.trim().toUpperCase().slice(0, 3) });
  };
  return (
    <span className={styles.parts} onBlur={(e) => !e.currentTarget.contains(e.relatedTarget) && done()}>
      <input className={`input ${styles.editor}`} type="number" step="any" autoFocus aria-label="Amount" value={amount} onChange={(e) => setAmount(e.target.value)} onKeyDown={keys(done, onCancel)} />
      <input className={`input ${styles.editor} ${styles.code}`} aria-label="Currency" placeholder="USD" maxLength={3} value={currency} onChange={(e) => setCurrency(e.target.value)} onKeyDown={keys(done, onCancel)} />
    </span>
  );
}

function MultiSelectEditor({ field, initial, onSave, onCancel }: { field: FieldMeta; initial: string[]; onSave: (v: unknown) => void; onCancel: () => void }) {
  const [chosen, setChosen] = useState(initial);
  const done = () => (chosen.join() === initial.join() ? onCancel() : onSave(chosen));
  return (
    <span className={styles.options} onBlur={(e) => !e.currentTarget.contains(e.relatedTarget) && done()} onKeyDown={keys(done, onCancel)}>
      {selectOptions(field).map((o, i) => (
        <label key={o.value} className={styles.option}>
          <input type="checkbox" autoFocus={i === 0} checked={chosen.includes(o.value)} onChange={(e) => setChosen(e.target.checked ? [...chosen, o.value] : chosen.filter((v) => v !== o.value))} />
          <span className={`${styles.chip} ${chipClass(o.color)}`}>{o.label}</span>
        </label>
      ))}
    </span>
  );
}

function RatingEditor({ initial, onSave, onCancel }: { initial: number; onSave: (v: unknown) => void; onCancel: () => void }) {
  return (
    <span className={styles.stars} role="radiogroup" aria-label="Rating" onBlur={(e) => !e.currentTarget.contains(e.relatedTarget) && onCancel()} onKeyDown={keys(onCancel, onCancel)}>
      {[1, 2, 3, 4, 5].map((n) => (
        <button key={n} type="button" role="radio" aria-checked={n === initial} aria-label={`${n} of 5`} className={n <= initial ? styles.starOn : styles.star} autoFocus={n === Math.max(initial, 1)} onClick={() => onSave(n === initial ? null : n)}>
          ★
        </button>
      ))}
    </span>
  );
}

/** A picker over the other object's records: type to narrow, pick one, or none. */
function RelationEditor({ slug, field, onSave, onCancel }: { slug: string; field: FieldMeta; onSave: (v: unknown) => void; onCancel: () => void }) {
  const [text, setText] = useState("");
  const target = field.options.relation?.target;
  const matches = useRecords(slug, target, { search: text.trim() || null, limit: 8 }, !!target);
  const rows = (matches.data?.records ?? []).map((r) => asRecord(r));
  const labelOf = (r: RecordJson): string => {
    const name = asRecord(r.name);
    const full = `${str(name.firstName)} ${str(name.lastName)}`.trim();
    return full || str(r.name) || str(r.title) || recordIdShort(r);
  };
  // The list floats over the page (the table scrolls and clips), anchored under the input.
  const input = useRef<HTMLInputElement>(null);
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    const box = input.current?.getBoundingClientRect();
    if (box) setAt({ left: box.left, top: box.bottom + 4 });
  }, []);
  return (
    <span className={styles.picker} onBlur={(e) => !e.currentTarget.contains(e.relatedTarget) && onCancel()}>
      <input ref={input} className={`input ${styles.editor}`} autoFocus placeholder={`Search ${field.label.toLowerCase()}…`} aria-label={field.label} value={text} onChange={(e) => setText(e.target.value)} onKeyDown={keys(() => rows[0] && onSave({ id: rows[0].id }), onCancel)} />
      <span className="glass menu" role="listbox" style={at ? { position: "fixed", left: at.left, top: at.top, right: "auto" } : undefined}>
        <button type="button" role="option" aria-selected={false} className="menuItem" onClick={() => onSave(null)}>
          None
        </button>
        {rows.map((r) => (
          <button key={str(r.id)} type="button" role="option" aria-selected={false} className="menuItem" onClick={() => onSave({ id: r.id })}>
            {labelOf(r)}
          </button>
        ))}
        {rows.length === 0 && <span className="ghostText menuItem">{matches.isPending ? "Looking…" : "No match"}</span>}
      </span>
    </span>
  );
}

function recordIdShort(r: RecordJson): string {
  return str(r.id).slice(0, 8);
}

