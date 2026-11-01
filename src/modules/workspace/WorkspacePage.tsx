import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useProfile } from "../../kernel/bridge/profile";
import { openSettings } from "../../kernel/bridge/settings";
import { useModules, useSetModules, type Modules } from "../../kernel/bridge/modules";
import { useCurrentWorkspace, useUpdateWorkspace } from "../../kernel/bridge/workspaces";
import OnOff from "../workspace-settings/OnOff";
import AgentsPanel from "../workspace-settings/AgentsPanel";
import BrandPanel from "../workspace-settings/BrandPanel";
import MeetingTypesPanel from "../workspace-settings/MeetingTypesPanel";
import PeoplePanel from "../workspace-settings/PeoplePanel";
import VocabularyPanel from "../workspace-settings/VocabularyPanel";
import DataModelPanel from "../workspace-settings/DataModelPanel";
import styles from "./WorkspacePage.module.css";

const route = getRouteApi("/shell/workspace");

export const WORKSPACE_TABS = [
  { id: "overview", label: "Overview" },
  { id: "brand", label: "Brand" },
  { id: "types", label: "Meeting types" },
  { id: "people", label: "People" },
  { id: "vocabulary", label: "Vocabulary" },
  { id: "agents", label: "Agents" },
  { id: "crm", label: "CRM" },
] as const;

export type WorkspaceTab = (typeof WORKSPACE_TABS)[number]["id"];

/** The current workspace's own settings — its brand, meeting types and people — in the main
 *  window. App-wide settings (permissions, backup, AI models) live in the Settings window. */
export default function WorkspacePage() {
  const { tab, object } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug ?? "";
  const show = (next: WorkspaceTab) => void navigate({ to: "/workspace", search: { tab: next }, replace: true });

  return (
    <div className="page">
      <p className="eyebrow">Workspace</p>
      <h1 className="pageTitle">{workspace?.name ?? ""}</h1>
      <p className="pageMeta">One company: its meetings, brand, minutes templates, people and agents stay together.</p>

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
          <Overview slug={slug} name={workspace.name} ownerTitle={workspace.owner_title} folder={workspace.folder_path} archived={workspace.archived} />
        ) : tab === "brand" ? (
          <BrandPanel key={slug} slug={slug} />
        ) : tab === "types" ? (
          <MeetingTypesPanel slug={slug} />
        ) : tab === "crm" ? (
          <DataModelPanel slug={slug} objectName={object} onOpen={(name) => void navigate({ to: "/workspace", search: { tab: "crm", object: name }, replace: true })} onBack={() => void navigate({ to: "/workspace", search: { tab: "crm" }, replace: true })} />
        ) : tab === "agents" ? (
          <AgentsPanel slug={slug} />
        ) : tab === "vocabulary" ? (
          <VocabularyPanel slug={slug} />
        ) : (
          <PeoplePanel slug={slug} />
        )}
      </div>
    </div>
  );
}

/** The modules this workspace has on (D-059). Off hides the module and pauses its agents; its
 *  data stays. */
function ModulesGroup({ slug }: { slug: string }) {
  const modules = useModules(slug).data;
  const set = useSetModules(slug);
  const row = (key: keyof Modules, label: string, hint: string) => (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        <p className="rowHint">{hint}</p>
      </div>
      <div className="rowControl">
        <OnOff label={label} value={modules?.[key]} disabled={!modules || set.isPending} onChange={(value) => modules && set.mutate({ ...modules, [key]: value })} />
      </div>
    </div>
  );
  return (
    <>
      <p className="groupTitle">Modules</p>
      <div className="group">
        {row("projects", "Projects", "The projects you follow; the Projects agent proposes their updates after each summary.")}
        {row("crm", "CRM", "The people you meet and their commitments. Names and spellings (the People tab) stay on either way.")}
      </div>
      <p className="fieldHint">Off hides the module and pauses its agents. Its data stays.</p>
      {set.error && (
        <p className="formError" role="alert">
          {set.error.message}
        </p>
      )}
    </>
  );
}

function Overview({ slug, name, ownerTitle, folder, archived }: { slug: string; name: string; ownerTitle: string; folder: string; archived: boolean }) {
  const update = useUpdateWorkspace();
  const [draft, setDraft] = useState(name);
  useEffect(() => setDraft(name), [name]);
  const [title, setTitle] = useState(ownerTitle);
  useEffect(() => setTitle(ownerTitle), [ownerTitle]);
  const profile = useProfile().data;
  const you = profile?.name ?? "";
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
            <p className="rowLabel">Your title</p>
            <p className="rowHint">{you ? `Your name, ${you}, comes from Settings → General.` : "Set your name in Settings → General."}</p>
          </div>
          <div className="rowControl">
            {profile && !you && (
              <button type="button" className="btn btnGhost btnCompact" onClick={() => void openSettings("general")}>
                Open Settings
              </button>
            )}
            <input
              className={`input ${styles.name}`}
              value={title}
              maxLength={100}
              placeholder="Not yet captured"
              aria-label="Your title"
              onChange={(e) => setTitle(e.target.value)}
              onBlur={() => title.trim() !== ownerTitle && update.mutate({ slug, patch: { owner_title: title.trim() } })}
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
            <p className="rowHint">Archived workspaces keep their data. Manage them in Settings.</p>
          </div>
          <div className="rowControl">
            <span className={archived ? "chip" : "chip chipPrimary"}>{archived ? "Archived" : "Active"}</span>
          </div>
        </div>
      </div>
      <ModulesGroup slug={slug} />
      {update.error && (
        <p className="formError" role="alert">
          {update.error.message}
        </p>
      )}
    </>
  );
}
