import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import { useCreateProject, useProjects, type Project } from "../../kernel/bridge/projects";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { STATUS_LABEL, milestoneText } from "./format";
import styles from "./Projects.module.css";

const route = getRouteApi("/shell/projects");

/** Projects (D-054): the projects you follow in this workspace. After each summary the Projects
 *  agent proposes their updates; nothing changes until you apply one on the project's page. */
export default function ProjectsPage() {
  const { data: workspace } = useCurrentWorkspace();
  const projects = useProjects(workspace?.slug);
  const { new: adding } = route.useSearch();
  const navigate = useNavigate();
  const list = projects.data;

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">Projects</h1>
        </div>
        <button type="button" className="btn btnPrimary" disabled={!workspace} onClick={() => void navigate({ to: "/projects", search: { new: true } })}>
          New project…
        </button>
      </div>
      <p className={`fieldHint ${styles.lead}`}>After each summary, New Dawn finds the projects a meeting touched and proposes their updates. Nothing changes until you apply them.</p>
      {projects.error && <p className="formError">{projects.error.message}</p>}
      {list?.length ? (
        <div className={styles.list}>
          {list.map((p) => (
            <ProjectRow key={p.id} project={p} />
          ))}
        </div>
      ) : (
        <div className={styles.list}>
          <p className="ghostText">{list ? "Not yet captured — add the projects you follow" : "Loading…"}</p>
          {[0, 1, 2].map((i) => (
            <div key={i} className={`${styles.slot} ghostCard`} />
          ))}
        </div>
      )}
      {adding && workspace && <NewProjectSheet slug={workspace.slug} onClose={() => void navigate({ to: "/projects", search: {} })} />}
    </div>
  );
}

function ProjectRow({ project: p }: { project: Project }) {
  const meta = [p.phase, milestoneText(p)].filter(Boolean).join(" · ");
  return (
    <Link to="/projects/$projectId" params={{ projectId: p.id }} className={`card ${styles.row}`}>
      <div className={styles.main}>
        <p className={styles.title}>{p.name}</p>
        <p className={styles.meta}>
          {meta || "No phase or milestone yet"}
          {p.meetings > 0 && ` · ${p.meetings} meeting${p.meetings === 1 ? "" : "s"}`}
          {p.risks.length > 0 && ` · ${p.risks.length} risk${p.risks.length === 1 ? "" : "s"}`}
        </p>
      </div>
      {p.waiting > 0 && (
        <span className="chip chipPrimary">
          {p.waiting} update{p.waiting === 1 ? "" : "s"} waiting
        </span>
      )}
      <span className="chip">{STATUS_LABEL[p.status]}</span>
    </Link>
  );
}

/** New project: a name, and what you know of its phase and next milestone. */
function NewProjectSheet({ slug, onClose }: { slug: string; onClose: () => void }) {
  const [name, setName] = useState("");
  const [phase, setPhase] = useState("");
  const [milestone, setMilestone] = useState("");
  const [due, setDue] = useState("");
  const create = useCreateProject(slug);
  const navigate = useNavigate();
  const dialog = useRef<HTMLDialogElement>(null);
  const close = () => !create.isPending && onClose();

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  function submit() {
    create.mutate(
      { name, phase, next_milestone: milestone, milestone_due: due || null },
      { onSuccess: (p) => void navigate({ to: "/projects/$projectId", params: { projectId: p.id } }) },
    );
  }

  return (
    <dialog
      ref={dialog}
      className="glass sheet"
      aria-labelledby="new-project-title"
      onCancel={(e) => {
        e.preventDefault();
        close();
      }}
      onPointerDown={(e) => e.target === e.currentTarget && close()}
    >
      <div className="sheetBody">
        <h2 id="new-project-title" className="sheetHeading">
          New project
        </h2>
        <label className="field">
          <span className="fieldLabel">Name</span>
          <input className="input" value={name} autoFocus onChange={(e) => setName(e.target.value)} placeholder="WiredScore certification" />
        </label>
        <label className="field">
          <span className="fieldLabel">Phase</span>
          <input className="input" value={phase} onChange={(e) => setPhase(e.target.value)} placeholder="Design, tender, construction…" />
        </label>
        <label className="field">
          <span className="fieldLabel">Next milestone</span>
          <input className="input" value={milestone} onChange={(e) => setMilestone(e.target.value)} placeholder="Audit visit" />
        </label>
        <label className="field">
          <span className="fieldLabel">Milestone date</span>
          <input className="input" type="date" value={due} onChange={(e) => setDue(e.target.value)} />
        </label>
        {create.error && <p className="formError">{create.error.message}</p>}
        <div className="sheetActions">
          <button type="button" className="btn btnGhost" disabled={create.isPending} onClick={onClose}>
            Cancel
          </button>
          <button type="button" className="btn btnPrimary" disabled={!name.trim() || create.isPending} onClick={submit}>
            {create.isPending ? "Adding…" : "Add project"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
