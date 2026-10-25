import { useState } from "react";
import DangerButton from "../../kernel/shell/DangerButton";
import type { Language } from "../../kernel/bridge/meetings";
import {
  checkTemplate,
  pickTemplateFile,
  useCreateMeetingType,
  useDeleteMeetingType,
  useMeetingTypeTemplates,
  useOpenTypeTemplate,
  useReplaceTypeTemplate,
  useResetTypeTemplate,
  useSetDefaultMeetingType,
  useTemplateFields,
  useUpdateMeetingType,
  type MeetingTypeView,
  type TemplateCheck,
  type TemplateField,
} from "../../kernel/bridge/templates";
import { LANGUAGE_LABEL } from "../meetings/format";
import styles from "./MeetingTypesPanel.module.css";
import settings from "./Settings.module.css";

const LANGUAGES: Language[] = ["en", "vi", "auto"];

/** Settings → Meeting types: per workspace, each type (weekly meeting, partner call…) with its
 *  own Word template for the minutes. */
export default function MeetingTypesPanel({ slug }: { slug: string }) {
  const types = useMeetingTypeTemplates(slug);

  return (
    <>
      <section className="section">
        <h2 className="sectionTitle">Meeting types</h2>
        <p className="sectionSubtitle">Each type fills its own Word template; a meeting's type picks the minutes it gets</p>
        {types.error && <p className={styles.error}>{types.error.message}</p>}
        <div className={styles.list}>
          {types.data?.length
            ? types.data.map((t) => <TypeRow key={`${t.id}:${t.name}`} slug={slug} type={t} only={types.data.length === 1} />)
            : [0, 1].map((i) => <div key={i} className={`${settings.slot} ghostCard`} />)}
        </div>
      </section>
      {slug && <AddType key={slug} slug={slug} />}
      <FieldReference />
    </>
  );
}

function TypeRow({ slug, type, only }: { slug: string; type: MeetingTypeView; only: boolean }) {
  const update = useUpdateMeetingType(slug);
  const makeDefault = useSetDefaultMeetingType(slug);
  const remove = useDeleteMeetingType(slug);
  const replace = useReplaceTypeTemplate(slug);
  const reset = useResetTypeTemplate(slug);
  const open = useOpenTypeTemplate(slug);
  const [name, setName] = useState(type.name);
  const [pending, setPending] = useState<{ path: string; check: TemplateCheck } | null>(null);
  const [pickError, setPickError] = useState<string | null>(null);
  const t = type.template;
  const error = update.error ?? makeDefault.error ?? remove.error ?? replace.error ?? reset.error ?? open.error;

  async function choose() {
    setPickError(null);
    replace.reset();
    try {
      const path = await pickTemplateFile();
      if (path) setPending({ path, check: await checkTemplate(path) });
    } catch (err) {
      setPickError(err instanceof Error ? err.message : String(err));
    }
  }

  const saveName = () => {
    if (name.trim() && name.trim() !== type.name) update.mutate({ id: type.id, name, language: type.language });
    else setName(type.name);
  };

  return (
    <div className={`card ${styles.row}`}>
      <div className={styles.head}>
        <div className={styles.identity}>
          <input
            className={`input ${styles.name}`}
            value={name}
            aria-label="Meeting type name"
            maxLength={100}
            onChange={(e) => setName(e.target.value)}
            onBlur={saveName}
            onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
          />
          <select
            className="input"
            aria-label="Spoken language"
            value={type.language}
            onChange={(e) => update.mutate({ id: type.id, name: name.trim() || type.name, language: e.target.value as Language })}
          >
            {LANGUAGES.map((l) => (
              <option key={l} value={l}>
                {LANGUAGE_LABEL[l]}
              </option>
            ))}
          </select>
          {type.is_default ? (
            <span className="chip chipPrimary">Default</span>
          ) : (
            <button type="button" className="btn btnGhost btnCompact" onClick={() => makeDefault.mutate(type.id)}>
              Make default
            </button>
          )}
        </div>
        {!only && (
          <DangerButton confirm="Delete this type?" onConfirm={() => remove.mutate(type.id)} disabled={remove.isPending} title="Its meetings keep everything but lose the type; the template goes to the Trash">
            Delete
          </DangerButton>
        )}
      </div>

      <div className={styles.template}>
        <div className={styles.templateState}>
          <p className="fieldLabel">Minutes template</p>
          {t.path ? (
            <p className="fieldHint">
              {t.modified_at ? `Changed ${new Date(t.modified_at).toLocaleDateString("en-GB")}` : "Own template"}
            </p>
          ) : (
            <p className="ghostText">Not yet captured — uses the default type's template</p>
          )}
          {t.check && <CheckSummary check={t.check} />}
        </div>
        <div className={styles.actions}>
          <button type="button" className="btn btnSecondary btnCompact" disabled={open.isPending} onClick={() => open.mutate({ id: type.id, reveal: false })}>
            Edit in Word
          </button>
          <button type="button" className="btn btnGhost btnCompact" disabled={replace.isPending} onClick={() => void choose()}>
            Replace…
          </button>
          {t.path && (
            <button type="button" className="btn btnGhost btnCompact" onClick={() => open.mutate({ id: type.id, reveal: true })}>
              Reveal in Finder
            </button>
          )}
          {t.has_starter && (
            <DangerButton confirm="Current one to the Trash?" onConfirm={() => reset.mutate(type.id)} disabled={reset.isPending}>
              Reset to starter
            </DangerButton>
          )}
        </div>
      </div>

      {pending && (
        <div className={styles.pending}>
          <p className={styles.pendingTitle}>{pending.path.split("/").pop()}</p>
          <CheckSummary check={pending.check} />
          <div className={styles.actions}>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setPending(null)}>
              Cancel
            </button>
            {!pending.check.error && (
              <button
                type="button"
                className="btn btnPrimary btnCompact"
                disabled={replace.isPending}
                onClick={() => replace.mutate({ id: type.id, path: pending.path }, { onSuccess: () => setPending(null) })}
              >
                Use this template
              </button>
            )}
          </div>
        </div>
      )}
      {(error ?? pickError) && (
        <p className={styles.error} role="alert">
          {error?.message ?? pickError}
        </p>
      )}
    </div>
  );
}

function AddType({ slug }: { slug: string }) {
  const create = useCreateMeetingType(slug);
  const [name, setName] = useState("");
  const [language, setLanguage] = useState<Language>("auto");
  const [file, setFile] = useState<string | null>(null);
  const [pickError, setPickError] = useState<string | null>(null);

  async function choose() {
    setPickError(null);
    try {
      setFile(await pickTemplateFile());
    } catch (err) {
      setPickError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <section className="section">
      <h2 className="sectionTitle">Add a meeting type</h2>
      <p className="sectionSubtitle">Starts as a copy of the default type's template, or a Word file you choose</p>
      <form
        className={styles.addForm}
        onSubmit={(e) => {
          e.preventDefault();
          create.mutate(
            { name, language, template: file },
            {
              onSuccess: () => {
                setName("");
                setFile(null);
              },
            },
          );
        }}
      >
        <label className={`field ${styles.grow}`}>
          <span className="fieldLabel">Name</span>
          <input className="input" value={name} maxLength={100} placeholder="Partner call" onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span className="fieldLabel">Spoken language</span>
          <select className="input" value={language} onChange={(e) => setLanguage(e.target.value as Language)}>
            {LANGUAGES.map((l) => (
              <option key={l} value={l}>
                {LANGUAGE_LABEL[l]}
              </option>
            ))}
          </select>
        </label>
        <button type="button" className="btn btnGhost" onClick={() => void choose()}>
          {file ? file.split("/").pop() : "Template: copy of default"}
        </button>
        <button type="submit" className="btn btnPrimary" disabled={!name.trim() || create.isPending}>
          Add
        </button>
      </form>
      {(create.error ?? pickError) && (
        <p className={styles.error} role="alert">
          {create.error?.message ?? pickError}
        </p>
      )}
    </section>
  );
}

function CheckSummary({ check }: { check: TemplateCheck }) {
  if (check.error) return <p className={styles.error}>Can't be used: {check.error}</p>;
  return (
    <>
      <p className="fieldHint">
        {check.fields.length} fields
        {check.unknown.length === 0 && " · all recognised"}
      </p>
      {check.unknown.length > 0 && (
        <p className={styles.warning}>Unknown, will come out empty: {check.unknown.map((f) => `{{${f}}}`).join(", ")}</p>
      )}
      {check.unused.length > 0 && <p className="fieldHint">Not used: {check.unused.join(", ")}</p>}
    </>
  );
}

/** Every field a template can use, for whoever edits it in Word. */
function FieldReference() {
  const fields = useTemplateFields();
  return (
    <section className="section">
      <details className={styles.reference}>
        <summary>Fields a template can use</summary>
        <p className="fieldHint">
          Type {"{{name}}"} for a value. A list repeats what sits between {"{{#name}}"} and {"{{/name}}"} — one paragraph, a
          table row, or several paragraphs — and inside it the item's own fields apply. Every list item also has {"{{n}}"} (1,
          2…) and {"{{nn}}"} (01, 02…).
        </p>
        {fields.data ? (
          <ul className={styles.fields}>
            {fields.data.map((f) => (
              <FieldItem key={f.name} field={f} />
            ))}
          </ul>
        ) : (
          <div className={`${settings.slot} ghostCard`} />
        )}
      </details>
    </section>
  );
}

function FieldItem({ field }: { field: TemplateField }) {
  const children = field.children.filter((c) => c.name !== "n" && c.name !== "nn");
  return (
    <li>
      <code>{field.list ? `{{#${field.name}}} … {{/${field.name}}}` : `{{${field.name}}}`}</code>
      {children.length > 0 && (
        <ul className={styles.fields}>
          {children.map((c) => (
            <FieldItem key={c.name} field={c} />
          ))}
        </ul>
      )}
    </li>
  );
}
