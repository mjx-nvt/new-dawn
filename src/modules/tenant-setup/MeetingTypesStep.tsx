import { useState } from "react";
import styles from "./MeetingTypesStep.module.css";

// SEE (stage A): mock meeting types. Stage B saves them; stage C pre-fills sections from the template headings.
const TYPES = [
  { name: "Partner call", language: "en", sections: 6 },
  { name: "Internal sync", language: "vi", sections: 4 },
  { name: "Delegation visit", language: "en", sections: 6 },
];

const SECTIONS = [
  { title: "Attendees", instruction: "Everyone present, with organisation and role." },
  { title: "Agenda", instruction: "The topics planned for the meeting, in order." },
  { title: "Discussion", instruction: "Key points per topic; who said what when it matters." },
  { title: "Decisions", instruction: "Each decision as one sentence." },
  { title: "Action items", instruction: "Task · owner · due date." },
  { title: "Next steps", instruction: "Follow-up meetings and open questions." },
];

export default function MeetingTypesStep() {
  const [open, setOpen] = useState(0);
  const [language, setLanguage] = useState("en");

  return (
    <section className="section">
      <h2 className="sectionTitle">Meeting types</h2>
      <p className="sectionSubtitle">Each type has its own minutes template, summary structure and email format</p>

      <div className={styles.list}>
        {TYPES.map((type, i) =>
          i !== open ? (
            <button key={type.name} type="button" className={`card ${styles.row}`} onClick={() => setOpen(i)}>
              <span className={styles.rowName}>{type.name}</span>
              <span className="chip">{type.language.toUpperCase()}</span>
              <span className="fieldHint">{type.sections} sections</span>
              <span className={styles.chevron} aria-hidden>›</span>
            </button>
          ) : (
            <div key={type.name} className={`card ${styles.editor}`}>
              <div className="fields">
                <label className="field">
                  <span className="fieldLabel">Name</span>
                  <input className="input" defaultValue={type.name} />
                </label>
                <div className="field">
                  <span className="fieldLabel">Default language</span>
                  <div className="segmented">
                    {[
                      ["en", "English"],
                      ["vi", "Tiếng Việt"],
                    ].map(([id, label]) => (
                      <button key={id} type="button" aria-pressed={language === id} onClick={() => setLanguage(id)}>
                        {label}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="field">
                  <span className="fieldLabel">Minutes template</span>
                  <div className={styles.template}>
                    <span className="fieldValue">TEMPLATE_Bien_ban_cuoc_hop.docx</span>
                    <button type="button" className="btn btnGhost btnCompact">Change…</button>
                  </div>
                </div>
                <label className={`field ${styles.check}`}>
                  <span className="fieldLabel">Transcript</span>
                  <span className={styles.checkRow}>
                    <input type="checkbox" /> Include the transcript in the minutes by default
                  </span>
                </label>
              </div>

              <div className={styles.block}>
                <div className={styles.blockHead}>
                  <span className="fieldLabel">Summary structure</span>
                  <span className="chip">From template headings</span>
                </div>
                <ol className={styles.sections}>
                  {SECTIONS.map((s, n) => (
                    <li key={s.title} className={styles.sectionRow}>
                      <span className={styles.index}>{n + 1}</span>
                      <input className="input" defaultValue={s.title} aria-label="Section title" />
                      <input className="input" defaultValue={s.instruction} aria-label="Instruction" />
                      <button type="button" className="btn btnGhost btnCompact" aria-label="Remove section">×</button>
                    </li>
                  ))}
                </ol>
                <button type="button" className="btn btnGhost btnCompact">+ Add section</button>
              </div>

              <div className={styles.block}>
                <span className="fieldLabel">Email format</span>
                <div className="fields">
                  <label className="field">
                    <span className="fieldHint">Subject</span>
                    <input className="input" defaultValue="[{tenant}] {title} — {date}" />
                  </label>
                  <label className="field">
                    <span className="fieldHint">Greeting</span>
                    <input className="input" defaultValue="Dear all," />
                  </label>
                </div>
                <div className={styles.include}>
                  <span className="fieldHint">Sections in the email</span>
                  <div className={styles.chips}>
                    {SECTIONS.map((s, n) => (
                      <label key={s.title} className={styles.checkRow}>
                        <input type="checkbox" defaultChecked={n > 1} /> {s.title}
                      </label>
                    ))}
                  </div>
                </div>
                <label className="field">
                  <span className="fieldHint">Sign-off</span>
                  <textarea className="input" defaultValue={"Best regards,\nTien Nguyen"} />
                </label>
              </div>
            </div>
          ),
        )}
        <button type="button" className={`btn btnGhost ${styles.add}`}>+ Add meeting type</button>
      </div>
    </section>
  );
}
