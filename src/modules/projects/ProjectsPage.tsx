import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useArchivePmProject, usePmProjects, useUpdatePmProject, type PmProject } from "../../kernel/bridge/pm";
import { usePeople } from "../../kernel/bridge/people";
import { useProjects } from "../../kernel/bridge/projects";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import MenuButton from "../../kernel/shell/MenuButton";
import NewProjectSheet from "./NewProjectSheet";
import { STATUS_LABEL } from "./format";
import { coverClass, whoName } from "./pm";
import styles from "./Projects.module.css";
import PageHeader from "../../kernel/shell/PageHeader";

const route = getRouteApi("/shell/projects");

/** Projects (Plane's list, D-073; the dossier's agent, D-054): cards or a list, searched,
 *  filtered and sorted; ★ favorites first; Archived apart. A project opens on its Overview —
 *  the updates its meetings brought — beside Plane's work. */
export default function ProjectsPage() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const search = route.useSearch();
  const navigate = useNavigate();
  const projects = usePmProjects(slug);
  const dossiers = useProjects(slug);
  const people = usePeople(slug ?? "").data ?? [];
  const [q, setQ] = useState("");
  const set = (patch: Partial<typeof search>) => void navigate({ to: "/projects", search: { ...search, ...patch }, replace: true });
  const waiting = new Map((dossiers.data ?? []).map((d) => [d.id, d.waiting]));
  const needle = q.trim().toLowerCase();
  const list = (projects.data ?? [])
    .filter((p) => (search.archived ? !!p.archived_at : !p.archived_at))
    .filter((p) => !search.network || p.network === search.network)
    .filter((p) => !needle || [p.name, p.identifier, p.description].some((t) => t.toLowerCase().includes(needle)))
    .sort((a, b) =>
      search.sort === "name" ? a.name.localeCompare(b.name) : search.sort === "created" ? b.created_at.localeCompare(a.created_at) : search.sort === "identifier" ? a.identifier.localeCompare(b.identifier) : Number(b.favorite) - Number(a.favorite),
    );
  const cards = search.layout !== "list";

  return (
    <div className="page">
      <PageHeader
        title={search.archived ? "Archived projects" : "Projects"}
        actions={
          <button type="button" className="btn btnPrimary" disabled={!workspace} onClick={() => set({ new: true })}>
            New project
          </button>
        }
      />
      <div className={styles.tools}>
        <input className={`input ${styles.search}`} type="search" placeholder="Search projects…" aria-label="Search projects" value={q} onChange={(e) => setQ(e.target.value)} />
        <select className="input" aria-label="Network" value={search.network ?? ""} onChange={(e) => set({ network: e.target.value === "private" || e.target.value === "public" ? e.target.value : undefined })}>
          <option value="">Any network</option>
          <option value="private">Private</option>
          <option value="public">Public</option>
        </select>
        <select className="input" aria-label="Order" value={search.sort ?? ""} onChange={(e) => set({ sort: e.target.value === "name" || e.target.value === "created" || e.target.value === "identifier" ? e.target.value : undefined })}>
          <option value="">Favorites first</option>
          <option value="name">Name</option>
          <option value="created">Newest</option>
          <option value="identifier">Identifier</option>
        </select>
        <button type="button" className={search.archived ? "btn btnSecondary btnCompact" : "btn btnGhost btnCompact"} aria-pressed={!!search.archived} onClick={() => set({ archived: search.archived ? undefined : true })}>
          Archived
        </button>
        <div className="segmented" role="group" aria-label="Layout">
          <button type="button" aria-pressed={cards} onClick={() => set({ layout: undefined })}>
            Cards
          </button>
          <button type="button" aria-pressed={!cards} onClick={() => set({ layout: "list" })}>
            List
          </button>
        </div>
      </div>
      {projects.error && <p className="formError">{projects.error.message}</p>}
      {list.length ? (
        <div className={cards ? styles.grid : styles.list}>
          {list.map((p) => (cards ? <ProjectCard key={p.id} slug={slug ?? ""} project={p} lead={whoName(p.lead, people)} waiting={waiting.get(p.id) ?? 0} /> : <ProjectLine key={p.id} slug={slug ?? ""} project={p} lead={whoName(p.lead, people)} waiting={waiting.get(p.id) ?? 0} />))}
        </div>
      ) : (
        <div className={cards ? styles.grid : styles.list}>
          {[0, 1, 2].map((i) => (
            <div key={i} className={`${styles.slot} ghostCard`}>
              {i === 0 && <span className="ghostText">{projects.isError ? "Couldn't be read" : !projects.data ? "Loading…" : search.archived ? "No archived project" : needle || search.network ? "No project matches" : "Not yet captured — New project starts one"}</span>}
            </div>
          ))}
        </div>
      )}
      {search.new && workspace && <NewProjectSheet slug={workspace.slug} onClose={() => set({ new: undefined })} />}
    </div>
  );
}

function Actions({ slug, project: p }: { slug: string; project: PmProject }) {
  const update = useUpdatePmProject(slug);
  const archive = useArchivePmProject(slug);
  const navigate = useNavigate();
  return (
    <span className={styles.actions}>
      <button type="button" className="btn btnGhost btnCompact" aria-label={p.favorite ? `Remove ${p.name} from favorites` : `Add ${p.name} to favorites`} aria-pressed={p.favorite} disabled={update.isPending} onClick={() => update.mutate({ id: p.id, patch: { favorite: !p.favorite } })}>
        {p.favorite ? "★" : "☆"}
      </button>
      <MenuButton
        label={`More for ${p.name}`}
        items={[
          { label: "Settings", onSelect: () => void navigate({ to: "/projects/$projectId", params: { projectId: p.id }, search: { tab: "settings" } }) },
          { label: p.archived_at ? "Restore" : "Archive", separator: true, onSelect: () => archive.mutate({ id: p.id, archived: !p.archived_at }) },
        ]}
      />
    </span>
  );
}

function Chips({ project: p, waiting }: { project: PmProject; waiting: number }) {
  return (
    <>
      <span className="chip">{p.identifier}</span>
      {p.status !== "active" && <span className="chip">{STATUS_LABEL[p.status]}</span>}
      {waiting > 0 && <span className="chip chipPrimary">{waiting === 1 ? "1 update waiting" : `${waiting} updates waiting`}</span>}
    </>
  );
}

function ProjectCard({ slug, project: p, lead, waiting }: { slug: string; project: PmProject; lead: string; waiting: number }) {
  return (
    <div className={`card ${styles.pcard}`}>
      <div className={`${styles.cover} ${coverClass(p.cover)}`} />
      <div className={styles.pcardBody}>
        <span className={styles.logo} aria-hidden>
          {p.emoji || p.identifier.slice(0, 2)}
        </span>
        <Link to="/projects/$projectId" params={{ projectId: p.id }} className={styles.pname}>
          {p.name}
        </Link>
        <p className={styles.chips}>
          <Chips project={p} waiting={waiting} />
        </p>
        <p className={styles.pdesc}>{p.description || <span className="ghostText">No description yet</span>}</p>
        <div className={styles.pfoot}>
          <span className="fieldHint">{[lead ? `Lead: ${lead}` : "No lead", p.network === "public" ? "Public" : "Private"].join(" · ")}</span>
          <Actions slug={slug} project={p} />
        </div>
      </div>
    </div>
  );
}

function ProjectLine({ slug, project: p, lead, waiting }: { slug: string; project: PmProject; lead: string; waiting: number }) {
  return (
    <div className={`card ${styles.pline}`}>
      <span className={styles.logo} aria-hidden>
        {p.emoji || p.identifier.slice(0, 2)}
      </span>
      <div className={styles.main}>
        <Link to="/projects/$projectId" params={{ projectId: p.id }} className={styles.pname}>
          {p.name}
        </Link>
        <p className={styles.meta}>{p.description || <span className="ghostText">No description yet</span>}</p>
      </div>
      <p className={styles.chips}>
        <Chips project={p} waiting={waiting} />
      </p>
      <span className="fieldHint">{[lead ? `Lead: ${lead}` : "No lead", p.network === "public" ? "Public" : "Private"].join(" · ")}</span>
      <Actions slug={slug} project={p} />
    </div>
  );
}
