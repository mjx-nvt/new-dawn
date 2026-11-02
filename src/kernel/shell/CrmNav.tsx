import { Link } from "@tanstack/react-router";
import { useState } from "react";
import { useCreateFavoriteFolder, useDeleteFavoriteFolder, useFavorites, useMoveFavorite, useNavigation, useRemoveFavorite, useSetNavigation, type Favorite, type NavItem } from "../bridge/navigation";
import nav from "./CrmNav.module.css";
import styles from "./Sidebar.module.css";

/** The CRM section of the sidebar (D-064, 0.10.7), as Twenty's: Favorites (records and views,
 *  in folders) and Workspace (the objects, in the order and folders the owner set; hidden ones
 *  left out). ✎ enters the customise mode: ▲ ▼ to reorder, eye to hide, folder names, × on a
 *  favorite; Save or Cancel. Nothing of it shows in the icon rail. */
export default function CrmNav({ slug, iconsOnly }: { slug: string | undefined; iconsOnly: boolean }) {
  const favorites = useFavorites(slug);
  const navigation = useNavigation(slug);
  const [editing, setEditing] = useState(false);
  if (iconsOnly || !slug) return null;
  const items = navigation.data ?? [];
  const shown = items.filter((i) => !i.is_hidden);
  const folders = favorites.data?.folders ?? [];
  const pins = favorites.data?.items ?? [];
  const byFolder = (folderId: string | null) => pins.filter((p) => (p.folder_id ?? null) === folderId);
  const objectFolders = [...new Set(shown.map((i) => i.folder).filter(Boolean))];

  if (editing) return <CrmNavEditor slug={slug} items={items} folders={folders} pins={pins} onDone={() => setEditing(false)} />;
  return (
    <div className={styles.section}>
      <div className={nav.head}>
        <p className={styles.sectionLabel}>Favorites</p>
        <button type="button" className={nav.pencil} aria-label="Customise the CRM section" title="Customise" onClick={() => setEditing(true)}>
          ✎
        </button>
      </div>
      {pins.length === 0 && <p className={`ghostText ${nav.empty}`}>Pin a record or a view with ★</p>}
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
      <p className={styles.sectionLabel}>Workspace</p>
      {shown
        .filter((i) => !i.folder)
        .map((i) => (
          <ObjectLink key={i.object_id} item={i} />
        ))}
      {objectFolders.map((name) => (
        <details key={name} className={nav.folder} open>
          <summary className={`${styles.item} ${nav.folderName}`}>
            <span className={styles.label}>{name}</span>
          </summary>
          {shown
            .filter((i) => i.folder === name)
            .map((i) => (
              <ObjectLink key={i.object_id} item={i} />
            ))}
        </details>
      ))}
    </div>
  );
}

function FavoriteLink({ favorite: p }: { favorite: Favorite }) {
  const mark = <span className={nav.mark}>{(p.label || "?").slice(0, 1).toUpperCase()}</span>;
  // A starred cycle or module (D-079): its project's tab, opened on it.
  if ((p.object === "pm-cycles" || p.object === "pm-modules") && p.view_id && p.record_id) {
    const search = p.object === "pm-cycles" ? { tab: "cycles", cycle: p.view_id } : { tab: "modules", module: p.view_id };
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

function ObjectLink({ item }: { item: NavItem }) {
  return (
    <Link to="/crm" search={{ tab: item.object }} className={styles.item} activeProps={{ className: styles.active }} activeOptions={{ includeSearch: true }} title={item.label}>
      <span className={nav.mark}>{item.label.slice(0, 1).toUpperCase()}</span>
      <span className={styles.label}>{item.label}</span>
    </Link>
  );
}

/** The customise mode: a working copy of the pins (folder, order) and the objects (order,
 *  hidden, folder), saved at once. */
function CrmNavEditor({ slug, items, folders, pins, onDone }: { slug: string; items: NavItem[]; folders: { id: string; name: string }[]; pins: Favorite[]; onDone: () => void }) {
  const setNavigation = useSetNavigation(slug);
  const moveFavorite = useMoveFavorite(slug);
  const removeFavorite = useRemoveFavorite(slug);
  const createFolder = useCreateFavoriteFolder(slug);
  const deleteFolder = useDeleteFavoriteFolder(slug);
  const [objects, setObjects] = useState(items);
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
    await setNavigation.mutateAsync(objects.map((o, i) => ({ object_id: o.object_id, position: i, is_hidden: o.is_hidden, folder: o.folder })));
    for (const [i, f] of favs.entries()) {
      const was = pins.find((p) => p.id === f.id);
      if (!was || was.folder_id !== f.folder_id || pins.indexOf(was) !== i) await moveFavorite.mutateAsync({ id: f.id, folderId: f.folder_id ?? null, position: i });
    }
    onDone();
  };
  const error = setNavigation.error ?? moveFavorite.error ?? removeFavorite.error ?? createFolder.error ?? deleteFolder.error;
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
      <p className={styles.sectionLabel}>Workspace</p>
      {objects.map((o, i) => (
        <div key={o.object_id} className={nav.editRow}>
          <span className={o.is_hidden ? `${nav.editLabel} ${nav.hidden}` : nav.editLabel}>{o.label}</span>
          <input className={`input ${nav.folderInput}`} aria-label={`Folder of ${o.label}`} placeholder="Folder" value={o.folder} onChange={(e) => setObjects(objects.map((x) => (x.object_id === o.object_id ? { ...x, folder: e.target.value } : x)))} />
          <button type="button" className={nav.tool} aria-label="Move up" disabled={i === 0} onClick={() => setObjects(move(objects, i, -1))}>▲</button>
          <button type="button" className={nav.tool} aria-label="Move down" disabled={i === objects.length - 1} onClick={() => setObjects(move(objects, i, 1))}>▼</button>
          <button type="button" className={nav.tool} aria-pressed={o.is_hidden} aria-label={o.is_hidden ? `Show ${o.label}` : `Hide ${o.label}`} onClick={() => setObjects(objects.map((x) => (x.object_id === o.object_id ? { ...x, is_hidden: !x.is_hidden } : x)))}>{o.is_hidden ? "◌" : "◉"}</button>
        </div>
      ))}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <div className={nav.editActions}>
        <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>Cancel</button>
        <button type="button" className="btn btnPrimary btnCompact" disabled={setNavigation.isPending || moveFavorite.isPending} onClick={() => void save()}>Save</button>
      </div>
    </div>
  );
}
