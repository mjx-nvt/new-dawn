import { useState, type FormEvent } from "react";
import {
  useCreateWorkspace,
  useCurrentWorkspace,
  useDeleteWorkspace,
  useSetCurrentWorkspace,
  useUpdateWorkspace,
  useWorkspaces,
  type Workspace,
} from "../../kernel/bridge/workspaces";
import settings from "./Settings.module.css";
import styles from "./WorkspacesPanel.module.css";

/** "VIFC Fintech Hub" → "vifc-fintech-hub"; strips Vietnamese diacritics. */
function suggestSlug(name: string): string {
  return name
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .replace(/đ/g, "d")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
}

/** Settings → Workspaces: create, switch, archive and trash workspaces. */
export default function WorkspacesPanel() {
  const workspaces = useWorkspaces();
  const { data: current } = useCurrentWorkspace();
  const create = useCreateWorkspace();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugEdited, setSlugEdited] = useState(false);

  const effectiveSlug = slugEdited ? slug : suggestSlug(name);

  function submit(event: FormEvent) {
    event.preventDefault();
    create.mutate(
      { name, slug: effectiveSlug },
      {
        onSuccess: () => {
          setName("");
          setSlug("");
          setSlugEdited(false);
        },
      },
    );
  }

  return (
    <>
      <section className={settings.section}>
        <h2 className={settings.sectionTitle}>New workspace</h2>
        <p className={settings.sectionSubtitle}>Creates the folder and its database</p>
        <form className={styles.form} onSubmit={submit}>
          <label className={styles.field}>
            <span className={settings.label}>Name</span>
            <input className={styles.input} value={name} onChange={(e) => setName(e.target.value)} placeholder="VIFC" />
          </label>
          <label className={styles.field}>
            <span className={settings.label}>Slug</span>
            <input
              className={styles.input}
              value={effectiveSlug}
              onChange={(e) => {
                setSlug(e.target.value);
                setSlugEdited(true);
              }}
              placeholder="vifc"
            />
          </label>
          <button className={styles.primary} type="submit" disabled={create.isPending}>
            {create.isPending ? "Creating…" : "Create"}
          </button>
        </form>
        {create.error && <p className={styles.error}>{create.error.message}</p>}
      </section>

      <section className={settings.section}>
        <h2 className={settings.sectionTitle}>All workspaces</h2>
        <p className={settings.sectionSubtitle}>Archived workspaces keep their data but their database is closed</p>
        {workspaces.error && <p className={styles.error}>{workspaces.error.message}</p>}
        <div className={styles.list}>
          {workspaces.data?.length
            ? workspaces.data.map((workspace) => (
                <WorkspaceRow key={workspace.id} workspace={workspace} isCurrent={workspace.slug === current?.slug} />
              ))
            : [0, 1].map((i) => <div key={i} className={`${settings.slot} ghostCard`} />)}
        </div>
      </section>
    </>
  );
}

function WorkspaceRow({ workspace, isCurrent }: { workspace: Workspace; isCurrent: boolean }) {
  const update = useUpdateWorkspace();
  const choose = useSetCurrentWorkspace();
  const remove = useDeleteWorkspace();
  const [confirming, setConfirming] = useState(false);
  const error = update.error ?? remove.error ?? choose.error;

  return (
    <div className={styles.card}>
      <div className={styles.cardMain}>
        <p className={styles.cardTitle}>
          {workspace.name} <span className={styles.slug}>{workspace.slug}</span>
          {isCurrent && <span className={styles.pillBrand}>Current</span>}
          {workspace.archived && <span className={styles.pill}>Archived</span>}
        </p>
        <p className={styles.path}>{workspace.folder_path}</p>
        {error && <p className={styles.error}>{error.message}</p>}
      </div>
      <div className={styles.actions}>
        {!isCurrent && !workspace.archived && (
          <button className={styles.ghost} type="button" disabled={choose.isPending} onClick={() => choose.mutate(workspace.slug)}>
            Switch
          </button>
        )}
        <button
          className={styles.ghost}
          type="button"
          disabled={update.isPending}
          onClick={() => update.mutate({ slug: workspace.slug, patch: { archived: !workspace.archived } })}
        >
          {workspace.archived ? "Unarchive" : "Archive"}
        </button>
        <button
          className={confirming ? styles.danger : styles.ghost}
          type="button"
          disabled={remove.isPending}
          onClick={() => (confirming ? remove.mutate(workspace.slug) : setConfirming(true))}
          onBlur={() => setConfirming(false)}
        >
          {confirming ? "Confirm: move to Trash" : "Move to Trash"}
        </button>
      </div>
    </div>
  );
}
