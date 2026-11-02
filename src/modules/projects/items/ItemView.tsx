import { Link, useNavigate } from "@tanstack/react-router";
import { Fragment, useRef, useState, type KeyboardEvent as ReactKeyboardEvent } from "react";
import { usePmLabels, usePmProjects } from "../../../kernel/bridge/pm";
import {
  itemFiles,
  openItemWindow,
  useArchivePmItem,
  useCommentMutations,
  useCopyPmItem,
  useCreatePmItem,
  useDeletePmItem,
  usePmActivity,
  usePmComments,
  usePmItem,
  usePmItemReactions,
  usePmSubItems,
  usePmVersions,
  useRestorePmVersion,
  useSubscribePmItem,
  useToggleReaction,
  useUpdatePmItem,
  useVotePmItem,
  type ItemPatch,
  type PmActivity,
  type PmComment,
  type PmItem,
  type Reaction,
} from "../../../kernel/bridge/pmItems";
import Editor from "../../../kernel/editor/Editor";
import { richPlain } from "../../../kernel/editor/plain";
import { formatDateTime } from "../../../kernel/format/format";
import DangerButton from "../../../kernel/shell/DangerButton";
import MenuButton from "../../../kernel/shell/MenuButton";
import { RELATIONS, asPrompt, duration, progress, sentence, transitions, useWho } from "./items";
import { Attachments, Links, RelationRow } from "./ItemTies";
import { DateField, Dot, EstimatePick, LabelPick, ParentPick, PeoplePick, PriorityMark, PriorityPick, StatePick } from "./Props";
import styles from "./Items.module.css";

const EMOJI = ["👍", "👎", "😄", "🎉", "😕", "❤️", "🚀", "👀"];
const TABS = ["all", "activity", "comments", "history", "transition"] as const;
type Tab = (typeof TABS)[number];
const TAB_LABEL: Record<Tab, string> = { all: "All", activity: "Activity", comments: "Comments", history: "History", transition: "Transition" };
/** An IME is still writing (Vietnamese): Enter confirms the composition, not the field. */
const composing = (e: ReactKeyboardEvent) => e.nativeEvent.isComposing || e.keyCode === 229;

function copy(text: string) {
  void navigator.clipboard.writeText(text).catch(() => undefined);
}

function Reactions({ reactions, disabled, onToggle }: { reactions: Reaction[]; disabled?: boolean; onToggle: (emoji: string) => void }) {
  const [open, setOpen] = useState(false);
  return (
    <div className={styles.reactions}>
      {reactions.map((r) => (
        <button key={r.emoji} type="button" className={`${styles.reaction} ${r.by.includes("me") ? styles.mine : ""}`} aria-pressed={r.by.includes("me")} disabled={disabled} onClick={() => onToggle(r.emoji)}>
          {r.emoji} {r.by.length}
        </button>
      ))}
      <button type="button" className="btn btnGhost btnCompact" aria-expanded={open} aria-label="Add a reaction" disabled={disabled} onClick={() => setOpen(!open)}>
        ☺︎+
      </button>
      {open &&
        EMOJI.map((e) => (
          <button
            key={e}
            type="button"
            className={styles.reaction}
            aria-label={`React ${e}`}
            onClick={() => {
              onToggle(e);
              setOpen(false);
            }}
          >
            {e}
          </button>
        ))}
    </div>
  );
}

function Comment({ slug, item, comment: c, replies, readOnly }: { slug: string; item: PmItem; comment: PmComment; replies: PmComment[]; readOnly: boolean }) {
  const who = useWho(slug);
  const m = useCommentMutations(slug, item.id);
  const react = useToggleReaction(slug, item.id);
  const [editing, setEditing] = useState(false);
  const [replying, setReplying] = useState(false);
  const draft = useRef<string>(JSON.stringify(c.body));
  return (
    <div className={c.parent_id ? `${styles.comment} ${styles.reply}` : styles.comment}>
      <p className={styles.commentHead}>
        <strong>{who(c.created_by)}</strong> · {formatDateTime(c.created_at)}
        {c.edited_at && <span className="fieldHint"> · edited</span>}
      </p>
      {editing ? (
        <>
          <div className={styles.editorBox}>
            <Editor value={c.body} home={{ slug, object: "pm-items", recordId: item.id, files: itemFiles(slug, item.id) }} onChange={(json) => (draft.current = json)} />
          </div>
          <div className={styles.inline}>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setEditing(false)}>
              Cancel
            </button>
            <button type="button" className="btn btnSecondary btnCompact" disabled={m.edit.isPending} onClick={() => m.edit.mutate({ id: c.id, body: draft.current }, { onSuccess: () => setEditing(false) })}>
              Save
            </button>
          </div>
        </>
      ) : (
        <div className={styles.commentBody}>
          <Editor key={JSON.stringify(c.body)} value={c.body} readOnly home={{ slug, object: "pm-items", recordId: item.id }} onChange={() => undefined} />
        </div>
      )}
      <div className={styles.inline}>
        <Reactions reactions={c.reactions} disabled={readOnly || react.isPending} onToggle={(emoji) => react.mutate({ commentId: c.id, emoji })} />
        {!readOnly && !c.parent_id && (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setReplying(!replying)}>
            Reply
          </button>
        )}
        {!readOnly && c.created_by === "me" && (
          <>
            <button type="button" className="btn btnGhost btnCompact" onClick={() => setEditing(true)}>
              Edit
            </button>
            <DangerButton confirm="Delete this comment?" disabled={m.remove.isPending} onConfirm={() => m.remove.mutate(c.id)}>
              Delete
            </DangerButton>
          </>
        )}
      </div>
      {(m.edit.error ?? m.remove.error) && <p className="formError">{(m.edit.error ?? m.remove.error)?.message}</p>}
      {replies.map((r) => (
        <Comment key={r.id} slug={slug} item={item} comment={r} replies={[]} readOnly={readOnly} />
      ))}
      {replying && <Composer slug={slug} item={item} parentId={c.id} onDone={() => setReplying(false)} />}
    </div>
  );
}

function Composer({ slug, item, parentId, onDone }: { slug: string; item: PmItem; parentId?: string; onDone?: () => void }) {
  const m = useCommentMutations(slug, item.id);
  const body = useRef<string>("");
  const [round, setRound] = useState(0);
  const [empty, setEmpty] = useState(true);
  return (
    <div className={styles.composer}>
      <div className={styles.editorBox}>
        <Editor
          key={round}
          value={null}
          placeholder={parentId ? "Reply…" : "Add a comment — @ to mention someone"}
          home={{ slug, object: "pm-items", recordId: item.id, files: itemFiles(slug, item.id) }}
          onChange={(json) => {
            body.current = json;
            setEmpty(!richPlain(json).trim() && !json.includes('"image"'));
          }}
        />
      </div>
      {m.add.error && <p className="formError">{m.add.error.message}</p>}
      <div className={styles.inline}>
        {onDone && (
          <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>
            Cancel
          </button>
        )}
        <button
          type="button"
          className="btn btnSecondary btnCompact"
          disabled={m.add.isPending || empty}
          onClick={() =>
            m.add.mutate(
              { body: body.current, parentId: parentId ?? null },
              {
                onSuccess: () => {
                  body.current = "";
                  setEmpty(true);
                  setRound(round + 1);
                  onDone?.();
                },
              },
            )
          }
        >
          {parentId ? "Reply" : "Comment"}
        </button>
      </div>
    </div>
  );
}

/** Two dashed rows where a list is empty (the ghost pattern); the first says why. */
function Ghosts({ text, className }: { text: string; className?: string }) {
  return (
    <>
      {[0, 1].map((n) => (
        <div key={n} className={`${className ?? styles.ghostRow} ghostCard`}>
          {n === 0 && <span className="ghostText">{text}</span>}
        </div>
      ))}
    </>
  );
}

function Activity({ slug, item, tab, onTab }: { slug: string; item: PmItem; tab: Tab; onTab: (t: Tab) => void }) {
  const who = useWho(slug);
  const acts = usePmActivity(slug, item.id).data ?? [];
  const comments = usePmComments(slug, item.id).data ?? [];
  const top = comments.filter((c) => !c.parent_id);
  const readOnly = !!item.archived_at;
  const thread = (c: PmComment) => <Comment key={c.id} slug={slug} item={item} comment={c} replies={comments.filter((r) => r.parent_id === c.id)} readOnly={readOnly} />;
  const line = (a: PmActivity) => (
    <li key={a.id} className={styles.act}>
      <span>{sentence(a, who)}</span>
      <span className="fieldHint">{formatDateTime(a.created_at)}</span>
    </li>
  );
  const nonComment = acts.filter((a) => a.kind !== "commented");
  return (
    <section className={styles.activity} aria-label="Activity">
      <div className="segmented" role="tablist" aria-label="Activity">
        {TABS.map((t) => (
          <button key={t} type="button" role="tab" aria-selected={t === tab} onClick={() => onTab(t)}>
            {TAB_LABEL[t]}
          </button>
        ))}
      </div>
      {tab === "all" && (
        <div className={styles.feed}>
          {[...nonComment.map((a) => ({ at: a.created_at, node: <ul className={styles.acts}>{line(a)}</ul>, key: a.id })), ...top.map((c) => ({ at: c.created_at, node: thread(c), key: c.id }))]
            .sort((x, y) => x.at.localeCompare(y.at))
            .map((e) => (
              <div key={e.key}>{e.node}</div>
            ))}
        </div>
      )}
      {tab === "activity" && <ul className={styles.acts}>{nonComment.map(line)}</ul>}
      {tab === "comments" && (
        <div className={styles.feed}>
          {top.map(thread)}
          {!top.length && <Ghosts text={readOnly ? "No comments" : "No comments yet — write the first below"} />}
        </div>
      )}
      {tab === "history" && (
        <ul className={styles.acts}>
          {nonComment.map((a) => (
            <li key={a.id} className={styles.history}>
              <span>
                <strong>{who(a.actor)}</strong> · {a.kind === "updated" ? a.field : a.kind} · <span className="fieldHint">{formatDateTime(a.created_at)}</span>
              </span>
              {a.kind === "updated" && a.field !== "description" && (
                <span className={styles.beforeAfter}>
                  <span className={styles.before}>{a.old_value || "—"}</span> → <span>{a.new_value || "—"}</span>
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
      {tab === "transition" && (
        <ul className={styles.acts}>
          {transitions(item.created_at, item.state_name, acts).map((t, i) => (
            <li key={`${t.from}-${i}`} className={styles.act}>
              <span>
                {t.state} — {duration(t.ms)}
                {t.current ? " so far" : ""}
              </span>
              <span className="fieldHint">since {formatDateTime(t.from)}</span>
            </li>
          ))}
        </ul>
      )}
      {(tab === "all" || tab === "comments") && !readOnly && <Composer slug={slug} item={item} />}
    </section>
  );
}

function SubItems({ slug, item, onOpen }: { slug: string; item: PmItem; onOpen: (id: string) => void }) {
  const subs = usePmSubItems(slug, item.id).data ?? [];
  const create = useCreatePmItem(slug);
  const [name, setName] = useState("");
  const share = progress(item);
  const add = () => name.trim() && create.mutate({ project_id: item.project_id, name, parent_id: item.id, assignees: [], labels: [] }, { onSuccess: () => setName("") });
  return (
    <section className={styles.subs} aria-label="Sub-work items">
      <div className={styles.subsHead}>
        <h3 className={styles.h3}>Sub-work items</h3>
        {share != null && (
          <span className={styles.progressWrap} aria-label={`${Math.round(share * 100)} % done`}>
            <span className={styles.progress}>
              <span style={{ width: `${Math.round(share * 100)}%` }} />
            </span>
            <span className="fieldHint">
              {item.sub_done} of {item.sub_items - item.sub_cancelled} done · {Math.round(share * 100)} %
            </span>
          </span>
        )}
      </div>
      <ul className={styles.subList}>
        {subs.map((s) => (
          <li key={s.id}>
            <button type="button" className={styles.subRow} onClick={() => onOpen(s.id)}>
              <Dot color={s.state_color} /> <span className={styles.key}>{s.key}</span> {s.name} <span className="fieldHint">{s.state_name}</span>
            </button>
          </li>
        ))}
      </ul>
      {!subs.length && <Ghosts text="No sub-work items yet" />}
      {!item.archived_at && (
        <div className={styles.inline}>
          <input className="input" aria-label="New sub-work item" placeholder="Add a sub-work item" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && !composing(e) && add()} />
          <button type="button" className="btn btnSecondary btnCompact" disabled={!name.trim() || create.isPending} onClick={add}>
            Add
          </button>
        </div>
      )}
      {create.error && <p className="formError">{create.error.message}</p>}
    </section>
  );
}

/** A work item (D-074, Plane's peek and page): its header actions, title, description (with
 *  its versions and reactions), properties, sub-items with their progress, and the activity
 *  tabs with the comments. Archived: read-only, with Restore. */
/** `onOpen` takes the item's project when it may be another one (a relation). */
export default function ItemView({ slug, itemId, tab, onTab, onOpen, onGone }: { slug: string; itemId: string; tab?: string; onTab: (tab: string | undefined) => void; onOpen: (id: string, projectId?: string) => void; onGone: () => void }) {
  const query = usePmItem(slug, itemId);
  const item = query.data;
  const update = useUpdatePmItem(slug);
  const archive = useArchivePmItem(slug);
  const remove = useDeletePmItem(slug);
  const copyItem = useCopyPmItem(slug);
  const subscribe = useSubscribePmItem(slug);
  const vote = useVotePmItem(slug);
  const react = useToggleReaction(slug, itemId);
  const reactions = usePmItemReactions(slug, itemId).data ?? [];
  const labels = usePmLabels(slug, item?.project_id ?? "").data ?? [];
  const projects = (usePmProjects(slug).data ?? []).filter((p) => !p.archived_at);
  const subs = usePmSubItems(slug, itemId).data ?? [];
  const who = useWho(slug);
  const navigate = useNavigate();
  const [versionsOpen, setVersionsOpen] = useState(false);
  const versions = usePmVersions(slug, itemId, versionsOpen);
  const restore = useRestorePmVersion(slug);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  if (!item) return <p className={query.isError ? "formError" : "ghostText"}>{query.isError ? query.error.message : "Reading…"}</p>;
  const ro = !!item.archived_at;
  const save = (patch: ItemPatch) => update.mutate({ id: item.id, patch });
  const error = update.error ?? archive.error ?? remove.error ?? copyItem.error ?? subscribe.error ?? restore.error ?? vote.error;
  const following = item.subscribers.includes("me");
  const closed = item.state_group === "completed" || item.state_group === "cancelled";
  const flash = (text: string) => {
    setNote(text);
    window.setTimeout(() => setNote(null), 1800);
  };
  return (
    <article className={styles.view}>
      <header className={styles.viewHead}>
        <span className={styles.key}>{item.key}</span>
        {item.parent_key && item.parent_id && (
          <button type="button" className="btn btnGhost btnCompact" onClick={() => item.parent_id && onOpen(item.parent_id)}>
            ↑ {item.parent_key}
          </button>
        )}
        <span className={styles.spacer} />
        {note && <span className="fieldHint">{note}</span>}
        <span className={styles.votes} role="group" aria-label="Your vote">
          <button type="button" className={`${styles.reaction} ${item.vote === 1 ? styles.mine : ""}`} aria-pressed={item.vote === 1} aria-label="Upvote" disabled={ro || vote.isPending} onClick={() => vote.mutate({ id: item.id, vote: item.vote === 1 ? 0 : 1 })}>
            ▲ {item.vote === 1 ? 1 : 0}
          </button>
          <button type="button" className={`${styles.reaction} ${item.vote === -1 ? styles.mine : ""}`} aria-pressed={item.vote === -1} aria-label="Downvote" disabled={ro || vote.isPending} onClick={() => vote.mutate({ id: item.id, vote: item.vote === -1 ? 0 : -1 })}>
            ▼ {item.vote === -1 ? 1 : 0}
          </button>
        </span>
        <button type="button" className="btn btnGhost btnCompact" aria-pressed={following} disabled={subscribe.isPending} onClick={() => subscribe.mutate({ id: item.id, on: !following })}>
          {following ? "Subscribed" : "Subscribe"}
        </button>
        <MenuButton
          label="Work item actions"
          items={[
            {
              label: "Copy ID",
              onSelect: () => {
                copy(item.key);
                flash(`${item.key} copied`);
              },
            },
            {
              label: "Copy as prompt",
              onSelect: () => {
                copy(asPrompt(item, labels, who, subs));
                flash("Copied as a prompt");
              },
            },
            { label: "Make a copy", separator: true, onSelect: () => copyItem.mutate({ id: item.id, projectId: null }, { onSuccess: (c) => onOpen(c.id) }) },
            ...projects.filter((p) => p.id !== item.project_id).slice(0, 8).map((p) => ({ label: `Copy to ${p.identifier} · ${p.name}`, onSelect: () => copyItem.mutate({ id: item.id, projectId: p.id }, { onSuccess: () => flash(`Copied to ${p.name}`) }) })),
            { label: "Open in a new window", separator: true, onSelect: () => void openItemWindow(item.project_id, item.id) },
            { label: "Open the full page", onSelect: () => void navigate({ to: "/projects/$projectId/items/$itemId", params: { projectId: item.project_id, itemId: item.id } }) },
            {
              label: ro ? "Restore" : closed ? "Archive" : "Archive (completed or cancelled only)",
              separator: true,
              disabled: !ro && !closed,
              onSelect: () => archive.mutate({ id: item.id, archived: !ro }),
            },
            { label: "Delete…", danger: true, onSelect: () => setConfirmDelete(true) },
          ]}
        />
      </header>
      {confirmDelete && (
        <div className={`card ${styles.ask}`} role="alertdialog" aria-labelledby="delete-item">
          <p id="delete-item" className={styles.askTitle}>
            Delete {item.key}?
          </p>
          <p className="fieldHint">{item.sub_items ? `Its ${item.sub_items} sub-work item${item.sub_items === 1 ? "" : "s"}, ` : "Its "}comments, activity and files go with it.</p>
          <div className={styles.inline}>
            <button type="button" className="btn btnGhost btnCompact" autoFocus onClick={() => setConfirmDelete(false)}>
              Cancel
            </button>
            <button type="button" className="btn btnDangerFilled btnCompact" disabled={remove.isPending} onClick={() => remove.mutate(item.id, { onSuccess: onGone })}>
              Delete
            </button>
          </div>
        </div>
      )}
      {ro && (
        <div className={styles.banner} role="status">
          <span>Archived — read-only.</span>
          <button type="button" className="btn btnSecondary btnCompact" disabled={archive.isPending} onClick={() => archive.mutate({ id: item.id, archived: false })}>
            Restore
          </button>
        </div>
      )}
      {error && (
        <p className="formError" role="alert">
          {error.message}
        </p>
      )}
      <input key={item.name} className={styles.title} aria-label="Title" defaultValue={item.name} readOnly={ro} maxLength={255} onBlur={(e) => e.target.value.trim() && e.target.value !== item.name && save({ name: e.target.value })} onKeyDown={(e) => e.key === "Enter" && !composing(e) && e.currentTarget.blur()} />
      <div className={styles.viewBody}>
        <div className={styles.viewMain}>
          <div className={styles.editorBox}>
            <Editor key={`${item.id}-${ro}`} value={item.description ?? null} readOnly={ro} placeholder="Add a description…" home={{ slug, object: "pm-items", recordId: item.id, files: itemFiles(slug, item.id) }} onChange={(json) => save({ description: json })} />
          </div>
          <div className={styles.inline}>
            <Reactions reactions={reactions} disabled={ro || react.isPending} onToggle={(emoji) => react.mutate({ commentId: null, emoji })} />
            <button type="button" className="btn btnGhost btnCompact" aria-expanded={versionsOpen} onClick={() => setVersionsOpen(!versionsOpen)}>
              Versions
            </button>
          </div>
          {versionsOpen && (
            <ul className={styles.versions} aria-label="Versions of the description">
              {(versions.data ?? []).map((v, i) => (
                <li key={v.id} className={styles.act}>
                  <span>
                    {formatDateTime(v.updated_at)} · {who(v.created_by)}
                    {i === 0 && <span className="chip"> Now</span>}
                    <span className={styles.excerpt}>{v.description_text.slice(0, 120) || "(empty)"}</span>
                  </span>
                  {i > 0 && !ro && (
                    <button type="button" className="btn btnSecondary btnCompact" disabled={restore.isPending} onClick={() => restore.mutate(v.id)}>
                      Restore
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
          {versionsOpen && !versions.data?.length && <Ghosts text={versions.isError ? "Couldn't be read" : versions.data ? "No versions yet — the description hasn't been written" : "Reading…"} />}
          <SubItems slug={slug} item={item} onOpen={onOpen} />
          <Links slug={slug} item={item} />
          <Attachments slug={slug} item={item} />
          <Activity slug={slug} item={item} tab={TABS.find((t) => t === tab) ?? "all"} onTab={(t) => onTab(t === "all" ? undefined : t)} />
        </div>
        <aside className={styles.side} aria-label="Properties">
          <dl className={styles.propList}>
            <dt>State</dt>
            <dd>
              <StatePick slug={slug} projectId={item.project_id} value={item.state_id} disabled={ro} onChange={(state_id) => save({ state_id })} />
            </dd>
            <dt>Priority</dt>
            <dd>
              <PriorityPick value={item.priority} disabled={ro} onChange={(priority) => save({ priority })} />
            </dd>
            <dt>Assignees</dt>
            <dd>
              <PeoplePick slug={slug} label="Assignees" value={item.assignees} disabled={ro} onChange={(assignees) => save({ assignees })} />
            </dd>
            <dt>Labels</dt>
            <dd>
              <LabelPick slug={slug} projectId={item.project_id} value={item.labels} disabled={ro} onChange={(l) => save({ labels: l })} />
            </dd>
            <dt>Start date</dt>
            <dd>
              <DateField label="Start date" value={item.start_date} disabled={ro} onSave={(start_date) => save({ start_date })} />
            </dd>
            <dt>Due date</dt>
            <dd>
              <DateField label="Due date" value={item.target_date} disabled={ro} onSave={(target_date) => save({ target_date })} />
            </dd>
            <dt>Estimate</dt>
            <dd>
              <EstimatePick slug={slug} projectId={item.project_id} value={item.estimate_point_id} disabled={ro} onChange={(estimate_point_id) => save({ estimate_point_id })} />
            </dd>
            <dt>Parent</dt>
            <dd>
              <ParentPick slug={slug} projectId={item.project_id} value={item.parent_id} self={item.id} disabled={ro} onChange={(parent_id) => save({ parent_id })} />
            </dd>
            {RELATIONS.map((r) => (
              <Fragment key={r.kind}>
                <dt>{r.label}</dt>
                <dd>
                  <RelationRow slug={slug} item={item} kind={r.kind} onOpen={onOpen} />
                </dd>
              </Fragment>
            ))}
            <dt>Cycle</dt>
            <dd className="ghostText">No cycles yet</dd>
            <dt>Modules</dt>
            <dd className="ghostText">No modules yet</dd>
            <dt>Created</dt>
            <dd className="fieldHint">
              {who(item.created_by)} · {formatDateTime(item.created_at)}
            </dd>
            <dt>Updated</dt>
            <dd className="fieldHint">
              {who(item.updated_by)} · {formatDateTime(item.updated_at)}
            </dd>
            {item.completed_at && (
              <>
                <dt>Completed</dt>
                <dd className="fieldHint">{formatDateTime(item.completed_at)}</dd>
              </>
            )}
          </dl>
          <p className={styles.sideMeta}>
            <PriorityMark value={item.priority} /> {item.comments ? `${item.comments} comment${item.comments === 1 ? "" : "s"}` : ""}
          </p>
          <Link to="/projects/$projectId" params={{ projectId: item.project_id }} search={{ tab: "items" }} className="fieldHint">
            All of the project's work items
          </Link>
        </aside>
      </div>
    </article>
  );
}
