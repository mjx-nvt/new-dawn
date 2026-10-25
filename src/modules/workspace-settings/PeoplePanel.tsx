import { useState } from "react";
import DangerButton from "../../kernel/shell/DangerButton";
import { useDeletePerson, useMergePeople, usePeople, useSavePerson, type Person } from "../../kernel/bridge/people";
import styles from "./PeoplePanel.module.css";
import settings from "./Settings.module.css";

/** "a, b ,, c" → ["a", "b", "c"] */
function splitAliases(text: string): string[] {
  return text
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean);
}

/** Settings → People: the people memory of a workspace — the right spelling, job title and
 *  organisation, and the other spellings that mean the same person. Summaries are corrected
 *  with it; saving a meeting's attendees adds to it. */
export default function PeoplePanel({ slug }: { slug: string }) {
  const [filter, setFilter] = useState("");
  const people = usePeople(slug);
  const q = filter.trim().toLowerCase();
  const shown = (people.data ?? []).filter(
    (p) => !q || [p.name, p.title, p.organisation, ...p.aliases].some((v) => v.toLowerCase().includes(q)),
  );

  return (
    <>
      <section className={settings.section}>
        <h2 className={settings.sectionTitle}>People</h2>
        <p className={settings.sectionSubtitle}>
          Remembered from the attendees you correct; every new summary uses these spellings, titles and organisations
        </p>
        <div className={styles.toolbar}>
          <label className={`field ${styles.grow}`}>
            <span className="fieldLabel">Find</span>
            <input className="input" value={filter} placeholder="Name, title, organisation…" onChange={(e) => setFilter(e.target.value)} />
          </label>
        </div>
        {people.error && <p className={styles.error}>{people.error.message}</p>}
        <div className={styles.list}>
          {people.data?.length ? (
            // Keyed by updated_at: a merge or a remembered attendee resets the draft, so a stale
            // one can't overwrite it.
            shown.map((p) => <PersonRow key={`${p.id}:${p.updated_at}`} slug={slug} person={p} others={people.data.filter((o) => o.id !== p.id)} />)
          ) : (
            <>
              <p className="ghostText">Not yet captured — save a meeting's attendees, or add someone below</p>
              {[0, 1].map((i) => (
                <div key={i} className={`${settings.slot} ghostCard`} />
              ))}
            </>
          )}
        </div>
      </section>
      {slug && <AddPerson key={slug} slug={slug} />}
    </>
  );
}

function PersonRow({ slug, person, others }: { slug: string; person: Person; others: Person[] }) {
  const save = useSavePerson(slug);
  const remove = useDeletePerson(slug);
  const merge = useMergePeople(slug);
  const [draft, setDraft] = useState({
    name: person.name,
    title: person.title,
    organisation: person.organisation,
    aliases: person.aliases.join(", "),
  });
  const [mergeWith, setMergeWith] = useState("");
  const changed =
    draft.name !== person.name ||
    draft.title !== person.title ||
    draft.organisation !== person.organisation ||
    draft.aliases !== person.aliases.join(", ");
  const error = save.error ?? remove.error ?? merge.error;

  const field = (key: keyof typeof draft, label: string) => (
    <label className="field">
      <span className="fieldLabel">{label}</span>
      <input className="input" value={draft[key]} onChange={(e) => setDraft({ ...draft, [key]: e.target.value })} />
    </label>
  );

  return (
    <div className={`card ${styles.row}`}>
      <div className={styles.grid}>
        {field("name", "Name")}
        {field("title", "Job title")}
        {field("organisation", "Organisation")}
        {field("aliases", "Also heard as (comma-separated)")}
      </div>
      <div className={styles.actions}>
        {others.length > 0 && (
          <>
            <select className="input" aria-label="Merge another person into this one" value={mergeWith} onChange={(e) => setMergeWith(e.target.value)}>
              <option value="">Same person as…</option>
              {others.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.name}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="btn btnGhost btnCompact"
              disabled={!mergeWith || merge.isPending}
              title="The other entry's name and spellings become spellings of this person"
              onClick={() => merge.mutate({ keep: person.id, other: mergeWith }, { onSuccess: () => setMergeWith("") })}
            >
              Merge into this
            </button>
          </>
        )}
        <span className={styles.grow} />
        <DangerButton confirm="Forget this person?" onConfirm={() => remove.mutate(person.id)} disabled={remove.isPending}>
          Forget
        </DangerButton>
        <button
          type="button"
          className="btn btnPrimary btnCompact"
          disabled={!changed || !draft.name.trim() || save.isPending}
          onClick={() =>
            save.mutate({
              id: person.id,
              name: draft.name,
              title: draft.title,
              organisation: draft.organisation,
              aliases: splitAliases(draft.aliases),
            })
          }
        >
          Save
        </button>
      </div>
      {error && (
        <p className={styles.error} role="alert">
          {error.message}
        </p>
      )}
    </div>
  );
}

function AddPerson({ slug }: { slug: string }) {
  const save = useSavePerson(slug);
  const empty = { name: "", title: "", organisation: "", aliases: "" };
  const [draft, setDraft] = useState(empty);
  const field = (key: keyof typeof draft, label: string, placeholder: string) => (
    <label className="field">
      <span className="fieldLabel">{label}</span>
      <input className="input" value={draft[key]} placeholder={placeholder} onChange={(e) => setDraft({ ...draft, [key]: e.target.value })} />
    </label>
  );
  return (
    <section className={settings.section}>
      <h2 className={settings.sectionTitle}>Add a person</h2>
      <p className={settings.sectionSubtitle}>For people you meet often, before they appear in a meeting</p>
      <form
        className={styles.row}
        onSubmit={(e) => {
          e.preventDefault();
          save.mutate(
            { id: null, name: draft.name, title: draft.title, organisation: draft.organisation, aliases: splitAliases(draft.aliases) },
            { onSuccess: () => setDraft(empty) },
          );
        }}
      >
        <div className={styles.grid}>
          {field("name", "Name", "Nguyễn Văn Tiến")}
          {field("title", "Job title", "Head of Hub")}
          {field("organisation", "Organisation", "VIFC")}
          {field("aliases", "Also heard as (comma-separated)", "Tien Nguyen, Mr Tien")}
        </div>
        <div className={styles.actions}>
          <span className={styles.grow} />
          <button type="submit" className="btn btnPrimary btnCompact" disabled={!draft.name.trim() || save.isPending}>
            Add
          </button>
        </div>
        {save.error && (
          <p className={styles.error} role="alert">
            {save.error.message}
          </p>
        )}
      </form>
    </section>
  );
}
