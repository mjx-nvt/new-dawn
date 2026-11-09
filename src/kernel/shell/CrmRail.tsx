import { useLocation } from "@tanstack/react-router";
import { useState } from "react";
import { useCrmProposals } from "../bridge/crmAgent";
import { useNavigation, useSetNavigation, type NavItem } from "../bridge/navigation";
import { SettingsIcon } from "./icons";
import ModuleRail, { RailGroup, RailLink, RailMark, railStyles as s } from "./ModuleRail";
import nav from "./Favorites.module.css";

/** Twenty's automation objects, grouped apart from the records. */
const AUTOMATION = ["dashboards", "workflows", "workflowRuns"];
/** The CRM's Tasks object stays out of the rail: New Dawn's Tasks is the one list (D-088); its
 *  table and board open from there, ⌘K and `g t`. */
const KEPT_OUT = ["tasks"];

/** The CRM module's rail (D-089; Twenty's section of D-064 moved out of the sidebar): the records
 *  in the owner's order and folders (hidden ones left out; ✎ reorders, hides, files them),
 *  Automation (Dashboards, Workflows, Workflow runs), New Dawn's Commitments and Waiting for you,
 *  and the module's settings. On a record's page its object's row stays lit. */
export default function CrmRail({ slug, compact }: { slug: string | undefined; compact: boolean }) {
  const location = useLocation();
  const navigation = useNavigation(slug);
  const waiting = useCrmProposals(slug).data?.length ?? 0;
  const [editing, setEditing] = useState(false);
  const items = (navigation.data ?? []).filter((i) => !KEPT_OUT.includes(i.object));
  const shown = items.filter((i) => !i.is_hidden);
  const records = shown.filter((i) => !AUTOMATION.includes(i.object));
  const automation = shown.filter((i) => AUTOMATION.includes(i.object));
  const folders = [...new Set(records.map((i) => i.folder).filter(Boolean))];
  // `/crm/people/<id>`: the record's object.
  const onRecord = /^\/crm\/([^/]+)\//.exec(location.pathname)?.[1];
  const row = (i: NavItem, part = false) => <RailLink key={i.object_id} to="/crm" search={{ tab: i.object }} activeOptions={{ includeSearch: true }} active={onRecord === i.object} icon={<RailMark>{i.label.slice(0, 1).toUpperCase()}</RailMark>} label={i.label} compact={compact} part={part} />;

  return (
    <ModuleRail label="CRM" compact={compact}>
      <RailGroup
        label="Records"
        control={
          <button type="button" className={s.pencil} aria-pressed={editing} aria-label="Customize the records" title="Customize" onClick={() => setEditing((was) => !was)}>
            ✎
          </button>
        }
      >
        {editing && !compact && slug ? (
          <ObjectsEditor slug={slug} items={items} onDone={() => setEditing(false)} />
        ) : (
          <>
            {records.filter((i) => !i.folder).map((i) => row(i))}
            {folders.map((name) => (
              <details key={name} className={s.folder} open>
                <summary className={s.row}>
                  <span className={s.label}>{name}</span>
                </summary>
                {records.filter((i) => i.folder === name).map((i) => row(i, true))}
              </details>
            ))}
          </>
        )}
      </RailGroup>
      {automation.length > 0 && <RailGroup label="Automation">{automation.map((i) => row(i))}</RailGroup>}
      <RailGroup label="New Dawn">
        <RailLink to="/crm" search={{ tab: "commitments" }} activeOptions={{ includeSearch: true }} icon={<RailMark>C</RailMark>} label="Commitments" compact={compact} />
        <RailLink to="/crm" search={{ tab: "waiting" }} activeOptions={{ includeSearch: true }} icon={<RailMark>W</RailMark>} label="Waiting for you" count={waiting} compact={compact} />
      </RailGroup>
      <RailGroup>
        <RailLink to="/workspace" search={{ tab: "crm" }} activeOptions={{ includeSearch: true }} icon={<SettingsIcon />} label="CRM settings" compact={compact} />
      </RailGroup>
    </ModuleRail>
  );
}

/** The customise mode (Twenty's): a working copy of the objects — order, hidden, folder — saved
 *  at once. */
function ObjectsEditor({ slug, items, onDone }: { slug: string; items: NavItem[]; onDone: () => void }) {
  const setNavigation = useSetNavigation(slug);
  const [objects, setObjects] = useState(items);
  const move = (i: number, d: -1 | 1) => {
    const j = i + d;
    if (j < 0 || j >= objects.length) return;
    const next = [...objects];
    [next[i], next[j]] = [next[j], next[i]];
    setObjects(next);
  };
  const save = () => setNavigation.mutate(objects.map((o, i) => ({ object_id: o.object_id, position: i, is_hidden: o.is_hidden, folder: o.folder })), { onSuccess: onDone });
  return (
    <div className={nav.editor}>
      {objects.map((o, i) => (
        <div key={o.object_id} className={nav.editRow}>
          <span className={o.is_hidden ? `${nav.editLabel} ${nav.hidden}` : nav.editLabel}>{o.label}</span>
          <input className={`input ${nav.folderInput}`} aria-label={`Folder of ${o.label}`} placeholder="Folder" value={o.folder} onChange={(e) => setObjects(objects.map((x) => (x.object_id === o.object_id ? { ...x, folder: e.target.value } : x)))} />
          <button type="button" className={nav.tool} aria-label="Move up" disabled={i === 0} onClick={() => move(i, -1)}>▲</button>
          <button type="button" className={nav.tool} aria-label="Move down" disabled={i === objects.length - 1} onClick={() => move(i, 1)}>▼</button>
          <button type="button" className={nav.tool} aria-pressed={o.is_hidden} aria-label={o.is_hidden ? `Show ${o.label}` : `Hide ${o.label}`} onClick={() => setObjects(objects.map((x) => (x.object_id === o.object_id ? { ...x, is_hidden: !x.is_hidden } : x)))}>
            {o.is_hidden ? "◌" : "◉"}
          </button>
        </div>
      ))}
      {setNavigation.error && (
        <p className="formError" role="alert">
          {setNavigation.error.message}
        </p>
      )}
      <div className={nav.editActions}>
        <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>Cancel</button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={setNavigation.isPending} onClick={save}>Save</button>
      </div>
    </div>
  );
}
