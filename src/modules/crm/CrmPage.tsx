import { getRouteApi, Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { useObject, useObjects } from "../../kernel/bridge/records";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import PeoplePage from "../people/PeoplePage";
import styles from "./Crm.module.css";
import RecordTable from "./RecordTable";

const route = getRouteApi("/shell/crm");

/** Twenty's objects as tabs — People, Companies, Opportunities, Tasks, Notes, each a record
 *  table (D-060) — and New Dawn's Commitments (D-055) beside them. The tab is URL state. */
export default function CrmPage() {
  const { tab } = route.useSearch();
  const navigate = useNavigate();
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const objects = useObjects(slug);
  const shown = (objects.data ?? []).filter((o) => !o.is_system && o.is_active);
  const current = tab === "commitments" ? undefined : (shown.find((o) => o.name_plural === tab) ?? shown[0]);
  const object = useObject(slug, current?.name_singular);
  const [search, setSearch] = useState("");
  const [deleted, setDeleted] = useState(false);
  const show = (next: string) => void navigate({ to: "/crm", search: { tab: next }, replace: true });
  const title = tab === "commitments" ? "Commitments" : (current?.label_plural ?? "CRM");

  return (
    <div className="page">
      <div className={styles.header}>
        <div>
          <p className="eyebrow">{workspace?.name ?? "No workspace"}</p>
          <h1 className="pageTitle">{title}</h1>
        </div>
        {tab === "commitments" ? (
          <Link to="/workspace" search={{ tab: "people" }} className="btn btnGhost">
            Names and spellings
          </Link>
        ) : (
          <div className={styles.tools}>
            <input className={`input ${styles.search}`} type="search" placeholder={`Search ${title.toLowerCase()}…`} aria-label={`Search ${title.toLowerCase()}`} value={search} onChange={(e) => setSearch(e.target.value)} />
            <button type="button" className={deleted ? "btn btnSecondary btnCompact" : "btn btnGhost btnCompact"} aria-pressed={deleted} onClick={() => setDeleted(!deleted)}>
              Deleted records
            </button>
          </div>
        )}
      </div>

      <div className={`segmented ${styles.tabs}`} role="tablist" aria-label="CRM">
        {shown.map((o) => (
          <button key={o.id} type="button" role="tab" aria-selected={tab !== "commitments" && o.id === current?.id} onClick={() => show(o.name_plural)}>
            {o.label_plural}
          </button>
        ))}
        <button type="button" role="tab" aria-selected={tab === "commitments"} onClick={() => show("commitments")}>
          Commitments
        </button>
      </div>

      <div role="tabpanel">
        {tab === "commitments" ? (
          <PeoplePage embedded />
        ) : slug && object.data ? (
          <RecordTable key={object.data.meta.id} slug={slug} object={object.data} search={search} deleted={deleted} />
        ) : (
          <div className={styles.tableWrap}>
            <div className={`${styles.slot} ghostCard`}>{objects.isError || object.isError ? <span className="ghostText">Couldn't be read — {(objects.error ?? object.error)?.message}</span> : null}</div>
          </div>
        )}
      </div>
    </div>
  );
}
