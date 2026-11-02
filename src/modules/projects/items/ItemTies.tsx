import { useState } from "react";
import { pickFiles } from "../../../kernel/bridge/files";
import { usePmPrefs } from "../../../kernel/bridge/notices";
import {
  openAttachmentFile,
  openLink,
  useAttachmentMutations,
  useLinkMutations,
  usePmAttachments,
  usePmLinks,
  usePmRelations,
  useRelationMutations,
  useSearchPmItems,
  type PmItem,
  type PmLink,
  type PmRelationKind,
} from "../../../kernel/bridge/pmItems";
import { formatDateTime, formatNumber } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import { RELATIONS } from "./items";
import { Dot } from "./Props";
import styles from "./Items.module.css";

const composing = (e: React.KeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

/** One relation row of the properties: the related items as chips (each opens), × to remove, +
 *  to find another item of any project by its key or title. */
export function RelationRow({ slug, item, kind, onOpen }: { slug: string; item: PmItem; kind: PmRelationKind; onOpen: (id: string, projectId?: string) => void }) {
  const relations = (usePmRelations(slug, item.id).data ?? []).filter((r) => r.kind === kind);
  const m = useRelationMutations(slug, item.id);
  const [finding, setFinding] = useState(false);
  const [query, setQuery] = useState("");
  const hits = useSearchPmItems(slug, query.trim(), item.id, finding).data ?? [];
  const label = RELATIONS.find((r) => r.kind === kind)?.label ?? kind;
  const ro = !!item.archived_at;
  return (
    <div className={styles.tie}>
      <span className={styles.chips}>
        {relations.map((r) => (
          <span key={r.id} className="chip">
            <Dot color={r.item.state_color} />
            <button type="button" className={styles.chipLink} title={r.item.name} onClick={() => onOpen(r.item.id, r.item.project_id)}>
              {r.item.key}
            </button>
            {!ro && (
              <button type="button" className={styles.chipX} aria-label={`Remove ${label.toLowerCase()} ${r.item.key}`} disabled={m.remove.isPending} onClick={() => m.remove.mutate(r.id)}>
                ×
              </button>
            )}
          </span>
        ))}
        {!relations.length && !finding && <span className="ghostText">None</span>}
        {!ro && (
          <button type="button" className="btn btnGhost btnCompact" aria-expanded={finding} aria-label={`Add ${label.toLowerCase()}`} onClick={() => setFinding(!finding)}>
            +
          </button>
        )}
      </span>
      {finding && (
        <div className={`glass ${styles.finder}`}>
          <input className="input" autoFocus aria-label={`Find a work item — ${label}`} placeholder="A key (VIFC-12) or words of a title" value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Escape" && !composing(e) && (e.preventDefault(), setFinding(false))} />
          <ul className={styles.hits}>
            {hits.map((h) => (
              <li key={h.id}>
                <button
                  type="button"
                  className={styles.hit}
                  disabled={m.add.isPending}
                  onClick={() =>
                    m.add.mutate(
                      { kind, otherId: h.id },
                      {
                        onSuccess: () => {
                          setFinding(false);
                          setQuery("");
                        },
                      },
                    )
                  }
                >
                  <Dot color={h.state_color} /> <span className={styles.key}>{h.key}</span> {h.name}
                </button>
              </li>
            ))}
            {!hits.length && <li className="ghostText">No work item by that key or title</li>}
          </ul>
        </div>
      )}
      {(m.add.error ?? m.remove.error) && <span className="formError">{(m.add.error ?? m.remove.error)?.message}</span>}
    </div>
  );
}

/** Two dashed rows where the list is empty; the first says why. */
function Ghosts({ text }: { text: string }) {
  return (
    <>
      {[0, 1].map((n) => (
        <div key={n} className={`${styles.ghostRow} ghostCard`}>
          {n === 0 && <span className="ghostText">{text}</span>}
        </div>
      ))}
    </>
  );
}

function LinkForm({ initial, busy, onSave, onCancel }: { initial?: PmLink; busy: boolean; onSave: (url: string, title: string) => void; onCancel?: () => void }) {
  const [url, setUrl] = useState(initial?.url ?? "");
  const [title, setTitle] = useState(initial?.title ?? "");
  const save = () => url.trim() && onSave(url, title);
  return (
    <div className={styles.inline}>
      <input className={`input ${styles.grow}`} aria-label="Link address" placeholder="Type or paste an address" value={url} onChange={(e) => setUrl(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && save()} />
      <input className="input" aria-label="Link title" placeholder="Title (optional)" value={title} maxLength={255} onChange={(e) => setTitle(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && save()} />
      {onCancel && (
        <button type="button" className="btn btnGhost btnCompact" onClick={onCancel}>
          Cancel
        </button>
      )}
      <button type="button" className="btn btnSecondary btnCompact" disabled={busy || !url.trim()} onClick={save}>
        {initial ? "Save" : "Add link"}
      </button>
    </div>
  );
}

/** Plane's Links: an address and a title, opened in the browser or Mail; edit, copy, remove. */
export function Links({ slug, item }: { slug: string; item: PmItem }) {
  const links = usePmLinks(slug, item.id);
  const m = useLinkMutations(slug, item.id);
  const [failed, setFailed] = useState<string | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const [round, setRound] = useState(0);
  const ro = !!item.archived_at;
  const list = links.data ?? [];
  return (
    <section className={styles.subs} aria-label="Links">
      <h3 className={styles.h3}>Links</h3>
      {list.map((l) =>
        editing === l.id ? (
          <LinkForm key={l.id} initial={l} busy={m.edit.isPending} onCancel={() => setEditing(null)} onSave={(url, title) => m.edit.mutate({ id: l.id, url, title }, { onSuccess: () => setEditing(null) })} />
        ) : (
          <div key={l.id} className={styles.tieRow}>
            <button type="button" className={styles.rowTitle} title={l.url} onClick={() => openLink(l.url).then(() => setFailed(null), (e: unknown) => setFailed(`Couldn't open it: ${String(e)}`))}>
              {l.title || l.url.replace(/^[a-z]+:\/\//i, "")}
            </button>
            <span className="fieldHint">{formatDateTime(l.created_at)}</span>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => void navigator.clipboard.writeText(l.url).catch(() => undefined)}>
              Copy
            </button>
            {!ro && (
              <>
                <button type="button" className="btn btnGhost btnCompact" onClick={() => setEditing(l.id)}>
                  Edit
                </button>
                <button type="button" className="btn btnGhost btnCompact btnDanger" disabled={m.remove.isPending} onClick={() => m.remove.mutate(l.id)}>
                  Remove
                </button>
              </>
            )}
          </div>
        ),
      )}
      {!list.length && <Ghosts text={links.isError ? "Couldn't be read" : links.data ? "No links yet" : "Reading…"} />}
      {!ro && <LinkForm key={round} busy={m.add.isPending} onSave={(url, title) => m.add.mutate({ url, title }, { onSuccess: () => setRound(round + 1) })} />}
      {(m.add.error ?? m.edit.error ?? m.remove.error) && <p className="formError">{(m.add.error ?? m.edit.error ?? m.remove.error)?.message}</p>}
      {failed && <p className="formError">{failed}</p>}
    </section>
  );
}

const size = (bytes: number) => (bytes >= 1024 * 1024 ? `${formatNumber(bytes / 1024 / 1024, 1)} MB` : `${formatNumber(Math.max(1, Math.round(bytes / 1024)), 0)} KB`);

/** Plane's Attachments: files copied into the workspace (up to the owner's limit), opened,
 *  shown in Finder, removed. */
export function Attachments({ slug, item }: { slug: string; item: PmItem }) {
  const files = usePmAttachments(slug, item.id);
  const m = useAttachmentMutations(slug, item.id);
  const [failed, setFailed] = useState<string | null>(null);
  const open = (a: Parameters<typeof openAttachmentFile>[0], reveal = false) => openAttachmentFile(a, reveal).then(() => setFailed(null), (e: unknown) => setFailed(`Couldn't open ${a.name}: ${String(e)}`));
  const limit = usePmPrefs().data?.attachment_mb ?? 5;
  const ro = !!item.archived_at;
  const list = files.data ?? [];
  const attach = async () => {
    const paths = await pickFiles();
    if (paths.length) m.add.mutate(paths);
  };
  return (
    <section className={styles.subs} aria-label="Attachments">
      <div className={styles.subsHead}>
        <h3 className={styles.h3}>Attachments</h3>
        <span className="fieldHint">Up to {limit} MB each (Settings → General)</span>
      </div>
      {list.map((a) => (
        <div key={a.id} className={styles.tieRow}>
          <button type="button" className={styles.rowTitle} onClick={() => void open(a)}>
            {a.name}
          </button>
          <span className="fieldHint">
            {size(a.size)} · {formatDateTime(a.created_at)}
          </span>
          <button type="button" className="btn btnGhost btnCompact" onClick={() => void open(a, true)}>
            Show in Finder
          </button>
          {!ro && (
            <DangerButton confirm={`Remove ${a.name}?`} disabled={m.remove.isPending} onConfirm={() => m.remove.mutate(a.id)}>
              Remove
            </DangerButton>
          )}
        </div>
      ))}
      {!list.length && <Ghosts text={files.isError ? "Couldn't be read" : files.data ? "No attachments yet" : "Reading…"} />}
      {!ro && (
        <div className={styles.inline}>
          <button type="button" className="btn btnSecondary btnCompact" disabled={m.add.isPending} onClick={() => void attach()}>
            {m.add.isPending ? "Attaching…" : "Attach"}
          </button>
        </div>
      )}
      {(m.add.error ?? m.remove.error) && <p className="formError">{(m.add.error ?? m.remove.error)?.message}</p>}
      {failed && <p className="formError">{failed}</p>}
    </section>
  );
}
