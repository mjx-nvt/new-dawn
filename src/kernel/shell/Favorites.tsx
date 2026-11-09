import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { useCreateFavoriteFolder, useDeleteFavoriteFolder, useFavorites, useMoveFavorite, useRemoveFavorite, type Favorite } from "../bridge/navigation";
import nav from "./Favorites.module.css";
import styles from "./Sidebar.module.css";

/** How many pins show open; past that the section starts folded. */
const FOLD_AT = 8;

/** The sidebar's Favorites (D-064, D-088): the one cross-module shortcut list — records, views,
 *  cycles, modules, pages and projects pinned with ★, in folders. ✎ enters the customise mode:
 *  folder, ▲ ▼ to reorder, × to unpin, folder names; Save or Cancel. Nothing of it shows in the
 *  icon rail. */
export default function Favorites({ slug, iconsOnly }: { slug: string | undefined; iconsOnly: boolean }) {
  const favorites = useFavorites(slug);
  const [editing, setEditing] = useState(false);
  if (iconsOnly || !slug) return null;
  const folders = favorites.data?.folders ?? [];
  const pins = favorites.data?.items ?? [];
  const byFolder = (folderId: string | null) => pins.filter((p) => (p.folder_id ?? null) === folderId);

  if (editing) return <FavoritesEditor slug={slug} folders={folders} pins={pins} onDone={() => setEditing(false)} />;
  return (
    <details className={`${styles.section} ${nav.favorites}`} open={pins.length <= FOLD_AT}>
      <summary className={nav.head}>
        <p className={styles.sectionLabel}>Favorites</p>
        <button type="button" className={nav.pencil} aria-label="Customise Favorites" title="Customise" onClick={(e) => { e.preventDefault(); setEditing(true); }}>
          ✎
        </button>
      </summary>
      {pins.length === 0 && <p className={`ghostText ${nav.empty}`}>Pin a record, a view or a project with ★</p>}
      {byFolder(null).map((p) => (
        <FavoriteLink key={p.id} favorite={p} />
      ))}
      {folders.map((f) => (
        <details key={f.id} className={nav.folder} open>
          <summary className={`${styles.item} ${nav.folderName}`}>
            <span className={styles.label}>{f.name}</span>
          </summary>
          {byFolder(f.id).map((p) => (
            <FavoriteLink key={p.id} favorite={p} />
          ))}
          {byFolder(f.id).length === 0 && <p className={`ghostText ${nav.empty}`}>Empty</p>}
        </details>
      ))}
    </details>
  );
}

function FavoriteLink({ favorite: p }: { favorite: Favorite }) {
  const mark = <span className={nav.mark}>{(p.label || "?").slice(0, 1).toUpperCase()}</span>;
  // A starred cycle, module or page (D-079, D-080): its project's tab, opened on it.
  if ((p.object === "pm-cycles" || p.object === "pm-modules" || p.object === "pm-pages") && p.view_id && p.record_id) {
    const search = p.object === "pm-cycles" ? { tab: "cycles", cycle: p.view_id } : p.object === "pm-modules" ? { tab: "modules", module: p.view_id } : { tab: "pages", page: p.view_id };
    return (
      <Link to="/projects/$projectId" params={{ projectId: p.record_id }} search={search} className={styles.item} activeProps={{ className: styles.active }} activeOptions={{ includeSearch: true }} title={`${p.label} · ${p.object_label}`}>
        {mark}
        <span className={styles.label}>{p.label}</span>
      </Link>
    );
  }
  // A Projects view (D-077): the workspace's views page, or its project's Work items.
  if (p.object === "pm-views" && p.view_id) {
    return p.record_id ? (
      <Link to="/projects/$projectId" params={{ projectId: p.record_id }} search={{ tab: "items", view: p.view_id }} className={styles.item} activeProps={{ className: styles.active }} activeOptions={{ includeSearch: true }} title={`${p.label} · Views`}>
        {mark}
        <span className={styles.label}>{p.label}</span>
      </Link>
    ) : (
      <Link to="/projects/views" search={{ view: p.view_id }} className={styles.item} activeProps={{ className: styles.active }} activeOptions={{ includeSearch: true }} title={`${p.label} · Views`}>
        {mark}
        <span className={styles.label}>{p.label}</span>
      </Link>
    );
  }
  return p.view_id ? (
    <Link to="/crm" search={{ tab: p.object, view: p.view_id }} className={styles.item} activeProps={{ className: styles.active }} activeOptions={{ includeSearch: true }} title={`${p.label} · ${p.object_label}`}>
      {mark}
      <span className={styles.label}>{p.label}</span>
    </Link>
  ) : (
    <Link to="/crm/$object/$recordId" params={{ object: p.object, recordId: p.record_id ?? "" }} className={styles.item} activeProps={{ className: styles.active }} title={`${p.label} · ${p.object_label}`}>
      {mark}
      <span className={styles.label}>{p.label}</span>
    </Link>
  );
}

/** The customise mode: a working copy of the pins (folder, order), saved at once; folders made
 *  and deleted as you go. */
function FavoritesEditor({ slug, folders, pins, onDone }: { slug: string; folders: { id: string; name: string }[]; pins: Favorite[]; onDone: () => void }) {
  const moveFavorite = useMoveFavorite(slug);
  const removeFavorite = useRemoveFavorite(slug);
  const createFolder = useCreateFavoriteFolder(slug);
  const deleteFolder = useDeleteFavoriteFolder(slug);
  const [favs, setFavs] = useState(pins);
  const [newFolder, setNewFolder] = useState("");
  const move = <T,>(list: T[], i: number, d: -1 | 1): T[] => {
    const j = i + d;
    if (j < 0 || j >= list.length) return list;
    const next = [...list];
    [next[i], next[j]] = [next[j], next[i]];
    return next;
  };
  const save = async () => {
    for (const [i, f] of favs.entries()) {
      const was = pins.find((p) => p.id === f.id);
      if (!was || was.folder_id !== f.folder_id || pins.indexOf(was) !== i) await moveFavorite.mutateAsync({ id: f.id, folderId: f.folder_id ?? null, position: i });
    }
    onDone();
  };
  const error = moveFavorite.error ?? removeFavorite.error ?? createFolder.error ?? deleteFolder.error;
  return (
    <div className={`${styles.section} ${nav.editor}`}>
      <p className={styles.sectionLabel}>Favorites</p>
      {favs.map((f, i) => (
        <div key={f.id} className={nav.editRow}>
          <span className={nav.editLabel}>{f.label}</span>
          <select className={`input ${nav.folderSelect}`} aria-label={`Folder of ${f.label}`} value={f.folder_id ?? ""} onChange={(e) => setFavs(favs.map((x) => (x.id === f.id ? { ...x, folder_id: e.target.value || null } : x)))}>
            <option value="">No folder</option>
            {folders.map((fo) => (
              <option key={fo.id} value={fo.id}>
                {fo.name}
              </option>
            ))}
          </select>
          <button type="button" className={nav.tool} aria-label="Move up" disabled={i === 0} onClick={() => setFavs(move(favs, i, -1))}>▲</button>
          <button type="button" className={nav.tool} aria-label="Move down" disabled={i === favs.length - 1} onClick={() => setFavs(move(favs, i, 1))}>▼</button>
          <button type="button" className={`${nav.tool} ${nav.toolDanger}`} aria-label={`Unpin ${f.label}`} onClick={() => removeFavorite.mutate(f.id, { onSuccess: () => setFavs(favs.filter((x) => x.id !== f.id)) })}>×</button>
        </div>
      ))}
      {folders.map((fo) => (
        <div key={fo.id} className={nav.editRow}>
          <span className={nav.editLabel}>
            {fo.name} <span className="ghostText">folder</span>
          </span>
          <button type="button" className={`${nav.tool} ${nav.toolDanger}`} aria-label={`Delete folder ${fo.name}`} onClick={() => deleteFolder.mutate(fo.id)}>×</button>
        </div>
      ))}
      <div className={nav.editRow}>
        <input
          className={`input ${nav.folderInput}`}
          aria-label="New folder"
          placeholder="New folder…"
          value={newFolder}
          onChange={(e) => setNewFolder(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && newFolder.trim()) createFolder.mutate(newFolder.trim(), { onSuccess: () => setNewFolder("") });
          }}
        />
      </div>
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <div className={nav.editActions}>
        <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>Cancel</button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={moveFavorite.isPending} onClick={() => void save()}>Save</button>
      </div>
    </div>
  );
}
