import { useModules } from "../bridge/modules";
import { useCurrentWorkspace } from "../bridge/workspaces";
import { WORKSPACE_TABS } from "../../modules/workspace/WorkspacePage";
import ModuleRail, { RailGroup, RailLink, RailMark } from "./ModuleRail";

/** The Workspace page's rail (D-089, D-090): its eight panes — Overview, Brand, Meeting types,
 *  People, Vocabulary, Agents, CRM, Projects (while the module is on, D-059) — as rows instead
 *  of a segmented control past five choices. The pane stays URL state (`?tab=`). */
export default function WorkspaceRail({ slug, compact }: { slug: string | undefined; compact: boolean }) {
  const { data: workspace } = useCurrentWorkspace();
  const projectsOn = useModules(slug).data?.projects !== false;
  return (
    <ModuleRail label="Workspace settings" compact={compact}>
      <RailGroup label={workspace?.name ?? "Workspace"}>
        {WORKSPACE_TABS.filter((t) => t.id !== "projects" || projectsOn).map((t) => (
          <RailLink key={t.id} to="/workspace" search={{ tab: t.id }} activeOptions={{ includeSearch: true }} icon={<RailMark>{t.label === "People" ? "Pe" : t.label === "Projects" ? "Pr" : t.label.slice(0, 1)}</RailMark>} label={t.label} compact={compact} />
        ))}
      </RailGroup>
    </ModuleRail>
  );
}
