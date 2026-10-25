import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useCurrentWorkspace, useUpdateWorkspace } from "../../kernel/bridge/workspaces";
import BrandPanel from "../workspace-settings/BrandPanel";
import MeetingTypesPanel from "../workspace-settings/MeetingTypesPanel";
import PeoplePanel from "../workspace-settings/PeoplePanel";
import styles from "./WorkspacePage.module.css";

const route = getRouteApi("/shell/workspace");

export const WORKSPACE_TABS = [
  { id: "overview", label: "Overview" },
  { id: "brand", label: "Brand" },
  { id: "types", label: "Meeting types" },
  { id: "people", label: "People" },
] as const;

export type WorkspaceTab = (typeof WORKSPACE_TABS)[number]["id"];

/** The current workspace's own settings — its brand, meeting types and people — in the main
 *  window. App-wide settings (permissions, backup, AI models) live in the Settings window. */
export default function WorkspacePage() {
  const { tab } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug ?? "";
  const show = (next: WorkspaceTab) => void navigate({ to: "/workspace", search: { tab: next }, replace: true });

  return (
    <div className="page">
      <p className="eyebrow">Workspace</p>
      <h1 className="pageTitle">{workspace?.name ?? ""}</h1>
      <p className="pageMeta">One company: its meetings, brand, minutes templates and people stay together.</p>

      <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="Workspace settings">
        {WORKSPACE_TABS.map((item) => (
          <button key={item.id} type="button" role="tab" aria-selected={item.id === tab} onClick={() => show(item.id)}>
            {item.label}
          </button>
        ))}
      </div>

      <div role="tabpanel" className={styles.panel}>
        {!workspace ? (
          <div className={`${styles.slot} ghostCard`} />
        ) : tab === "overview" ? (
          <Overview slug={slug} name={workspace.name} folder={workspace.folder_path} archived={workspace.archived} />
        ) : tab === "brand" ? (
          <BrandPanel slug={slug} />
        ) : tab === "types" ? (
          <MeetingTypesPanel slug={slug} />
        ) : (
          <PeoplePanel slug={slug} />
        )}
      </div>
    </div>
  );
}

function Overview({ slug, name, folder, archived }: { slug: string; name: string; folder: string; archived: boolean }) {
  const update = useUpdateWorkspace();
  const [draft, setDraft] = useState(name);
  useEffect(() => setDraft(name), [name]);
  return (
    <>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Name</p>
            <p className="rowHint">Shown in the sidebar and on documents as the organisation.</p>
          </div>
          <div className="rowControl">
            <input
              className={`input ${styles.name}`}
              value={draft}
              maxLength={100}
              aria-label="Workspace name"
              onChange={(e) => setDraft(e.target.value)}
              onBlur={() => draft.trim() && draft.trim() !== name && update.mutate({ slug, patch: { name: draft.trim() } })}
              onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && e.currentTarget.blur()}
            />
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Folder</p>
            <p className="rowHint">{folder}</p>
          </div>
          <div className="rowControl">
            <span className="chip">{slug}</span>
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Status</p>
            <p className="rowHint">Archived workspaces keep their data; their database is closed. Manage all workspaces in Settings.</p>
          </div>
          <div className="rowControl">
            <span className={archived ? "chip" : "chip chipBrand"}>{archived ? "Archived" : "Active"}</span>
          </div>
        </div>
      </div>
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
    </>
  );
}
