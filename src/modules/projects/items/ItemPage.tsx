import { getRouteApi, useNavigate } from "@tanstack/react-router";
import { useCurrentWorkspace } from "../../../kernel/bridge/workspaces";
import ItemView from "./ItemView";

const route = getRouteApi("/shell/projects/$projectId/items/$itemId");

/** A work item's full page (D-074) — also what "Open in a new window" shows. */
export default function ItemPage() {
  const { projectId, itemId } = route.useParams();
  const { activity } = route.useSearch();
  const { data: workspace } = useCurrentWorkspace();
  const navigate = useNavigate();
  if (!workspace) return null;
  return (
    <div className="page">
      <ItemView
        key={itemId}
        slug={workspace.slug}
        itemId={itemId}
        tab={activity}
        onTab={(tab) => void navigate({ to: "/projects/$projectId/items/$itemId", params: { projectId, itemId }, search: tab ? { activity: tab } : {}, replace: true })}
        onOpen={(id) => void navigate({ to: "/projects/$projectId/items/$itemId", params: { projectId, itemId: id } })}
        onGone={() => void navigate({ to: "/projects/$projectId", params: { projectId }, search: { tab: "items" } })}
      />
    </div>
  );
}
