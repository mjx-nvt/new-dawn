import { useNavigate } from "@tanstack/react-router";
import { Fragment, useEffect, useRef, useState, type ReactNode } from "react";
import { useAskAssistant, useAssistantAgents, useChatMessages, useChatThreads, useDeleteChatThread, useNewChatThread, useProposalAction, type ChatMessage } from "../../kernel/bridge/assistant";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import { SparkleIcon } from "../../kernel/shell/icons";
import MenuButton from "../../kernel/shell/MenuButton";
import AiProgress from "../meetings/AiProgress";
import { formatWhen } from "../meetings/format";
import s from "./Assistant.module.css";

export interface AskTarget {
  object: string;
  id: string;
  /** The record's name, and its object's ("company"), for the panel's words. */
  label?: string;
  objectLabel?: string;
}

const EXAMPLES = ["Which opportunities above 50k have no activity for 30 days?", "Which companies have had no activity for 60 days?", "What's the total amount of the opportunities in Proposal?"];

/** A record link in an answer ("/crm/<object>/<id>") opened in the app. */
function useOpen() {
  const navigate = useNavigate();
  return (path: string) => {
    const m = /^\/crm\/([^/]+)\/([^/?#]+)$/.exec(path);
    if (m?.[1] && m[2]) void navigate({ to: "/crm/$object/$recordId", params: { object: m[1], recordId: m[2] }, search: {} });
  };
}

/** `[label](/crm/…)` and `**bold**` in a line. */
function inline(text: string, open: (path: string) => void): ReactNode[] {
  const out: ReactNode[] = [];
  const re = /\[([^\]]+)\]\(([^)\s]+)\)|\*\*([^*]+)\*\*/g;
  let last = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const [, label, href, bold] = m;
    if (label !== undefined && href !== undefined) {
      out.push(
        href.startsWith("/crm/") ? (
          <a key={m.index} href={href} onClick={(e) => (e.preventDefault(), open(href))}>
            {label}
          </a>
        ) : (
          label
        ),
      );
    } else out.push(<strong key={m.index}>{bold}</strong>);
    last = re.lastIndex;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** An answer's words: paragraphs, bulleted or numbered lists, links to records, bold. */
function Rich({ text, open }: { text: string; open: (path: string) => void }) {
  const item = /^\s*([-*•]|\d+[.)])\s+/;
  return (
    <>
      {text.split(/\n{2,}/).map((block, i) => {
        const lines = block.split("\n").filter((l) => l.trim());
        if (lines.length && lines.every((l) => item.test(l)))
          return (
            <ul key={i}>
              {lines.map((l, j) => (
                <li key={j}>{inline(l.replace(item, ""), open)}</li>
              ))}
            </ul>
          );
        return (
          <p key={i}>
            {lines.map((l, j) => (
              <Fragment key={j}>
                {j > 0 && <br />}
                {inline(l, open)}
              </Fragment>
            ))}
          </p>
        );
      })}
    </>
  );
}

function asObject(v: unknown): Record<string, unknown> {
  return v && typeof v === "object" && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
}

/** A lookup in a line: what the agent looked at. */
function lookedUp(m: ChatMessage): string {
  const r = asObject(m.tool);
  if (typeof r.error === "string") return `Couldn't ${m.tool_name.replace(/_/g, " ")} — ${r.error}`;
  const what = typeof r.object === "string" ? r.object : "records";
  switch (m.tool_name) {
    case "find_records":
      return `Looked up ${what} records — ${String(r.total ?? 0)} found`;
    case "count_records":
      return `Counted ${what} records — ${String(r.count ?? 0)}`;
    case "aggregate":
      return `${String(r.op ?? "").toLowerCase()} of ${String(r.field ?? "")} — ${String(r.value ?? "")}`;
    case "open_record":
      return `Opened ${String(r.label ?? "a record")}`;
    default:
      return m.tool_name;
  }
}

function Proposal({ m, busy, onAct, open }: { m: ChatMessage; busy: boolean; onAct: (action: "apply" | "dismiss") => void; open: (path: string) => void }) {
  const p = asObject(m.proposal);
  const object = typeof p.object === "string" ? p.object : "";
  const plural = typeof p.plural === "string" ? p.plural : object;
  const made = m.proposal_status === "APPLIED" ? `/crm/${plural}/${m.proposal_result}` : "";
  return (
    <div className={s.proposal}>
      <p className="eyebrow">Proposed</p>
      <span className={s.proposalText}>{typeof p.summary === "string" ? p.summary : "A change"}</span>
      {m.proposal_status === "PROPOSED" && m.proposal_result && (
        <p className="formError" role="alert">
          {m.proposal_result}
        </p>
      )}
      <div className={s.proposalActions}>
        {m.proposal_status === "PROPOSED" ? (
          <>
            <button type="button" className="btn btnSecondary btnCompact" disabled={busy} onClick={() => onAct("apply")}>
              Apply
            </button>
            <button type="button" className="btn btnGhost btnCompact" disabled={busy} onClick={() => onAct("dismiss")}>
              Dismiss
            </button>
          </>
        ) : m.proposal_status === "APPLYING" ? (
          <span className="fieldHint">Applying…</span>
        ) : m.proposal_status === "APPLIED" ? (
          <>
            <span className="chip">Applied</span>
            {made && (
              <button type="button" className="btn btnGhost btnCompact" onClick={() => open(made)}>
                Open
              </button>
            )}
          </>
        ) : (
          <span className="chip">Dismissed</span>
        )}
      </div>
    </div>
  );
}

/** The assistant (D-070): a glass panel on the right with the chats — a record's when it was
 *  opened on one — the agent's answers with links to the records it found, its lookups in a
 *  line each, and the changes it proposes as cards to apply. Esc closes. */
export default function ChatPanel({ on, onClose }: { on: AskTarget | null; onClose: () => void }) {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const place = on ? { object: on.object, id: on.id } : null;
  const threads = useChatThreads(slug, place);
  const agents = useAssistantAgents(slug);
  // null: the newest chat of this place; "new": a chat to start.
  const [choice, setChoice] = useState<string | null>(null);
  const [agentId, setAgentId] = useState("");
  const [text, setText] = useState("");
  const [pending, setPending] = useState<{ text: string; at: number } | null>(null);
  const [trouble, setTrouble] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const create = useNewChatThread(slug);
  const ask = useAskAssistant(slug);
  const remove = useDeleteChatThread(slug);
  const threadId = choice === "new" ? null : (choice ?? threads.data?.[0]?.id ?? null);
  const messages = useChatMessages(slug, threadId);
  const act = useProposalAction(slug, threadId);
  const open = useOpen();
  const end = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    setChoice(null);
    setTrouble(null);
    setDeleting(false);
  }, [slug, on?.object, on?.id]);
  useEffect(() => end.current?.scrollIntoView({ block: "end" }), [messages.data, pending]);
  useEffect(() => input.current?.focus(), [threadId]);

  const send = async (question?: string) => {
    const q = (question ?? text).trim();
    if (!q || !slug || pending) return;
    setTrouble(null);
    setText("");
    setPending({ text: q, at: Date.now() });
    try {
      const id = threadId ?? (await create.mutateAsync({ agentId: agentId || null, on: place })).id;
      setChoice(id);
      await ask.mutateAsync({ threadId: id, text: q });
    } catch (e) {
      setTrouble(e instanceof Error ? e.message : String(e));
      setText(q);
    } finally {
      setPending(null);
    }
  };
  const current = threads.data?.find((t) => t.id === threadId);
  const shown = (messages.data ?? []).filter((m) => m.role !== "assistant" || m.content.trim());
  const title = on ? `About ${on.label || current?.record_label || "this record"}` : current?.title || "New chat";
  return (
    <aside className={`glass ${s.panel}`} role="dialog" aria-label="Ask AI" onKeyDown={(e) => e.key === "Escape" && (e.preventDefault(), onClose())}>
      <div className={s.head}>
        <div>
          <p className="eyebrow">Ask AI</p>
          <h2 className={s.title}>{title}</h2>
        </div>
        <div className={s.headActions}>
          <MenuButton
            label="More"
            items={[
              { label: "New chat", onSelect: () => setChoice("new") },
              { label: "Delete this chat…", danger: true, separator: true, disabled: !threadId, onSelect: () => setDeleting(true) },
            ]}
          />
          <button type="button" className="btn btnGhost btnCompact" aria-label="Close" onClick={onClose}>
            ✕
          </button>
        </div>
      </div>
      {deleting && threadId && (
        <div className={s.confirm} role="alertdialog" aria-label="Delete this chat?">
          <span>Delete this chat? Its questions and answers go; applied changes stay.</span>
          <button type="button" className="btn btnDangerFilled btnCompact" disabled={remove.isPending} onClick={() => remove.mutate(threadId, { onSuccess: () => (setDeleting(false), setChoice(null)) })}>
            Delete
          </button>
          <button type="button" className="btn btnGhost btnCompact" onClick={() => setDeleting(false)}>
            Cancel
          </button>
        </div>
      )}
      <div className={s.pick}>
        <select className="input" aria-label="Chat" value={threadId ?? "new"} onChange={(e) => setChoice(e.target.value)}>
          <option value="new">New chat</option>
          {(threads.data ?? []).map((t) => (
            <option key={t.id} value={t.id}>
              {(t.title || "Untitled") + " · " + formatWhen(t.updated_at)}
            </option>
          ))}
        </select>
        {!threadId && (agents.data?.length ?? 0) > 1 && (
          <select className="input" aria-label="Agent" value={agentId} onChange={(e) => setAgentId(e.target.value)}>
            {(agents.data ?? []).map((a) => (
              <option key={a.id} value={a.is_default ? "" : a.id}>
                {a.name}
              </option>
            ))}
          </select>
        )}
      </div>
      <div className={s.body} aria-live="polite">
        {!shown.length && !pending && (
          <div className={s.intro}>
            <p className="fieldHint">The assistant looks things up in your CRM with the local model and proposes changes for you to apply. Try:</p>
            {(on ? [`Summarise this ${on.objectLabel ?? "record"}`, `What changed on this ${on.objectLabel ?? "record"} lately?`] : EXAMPLES).map((ex) => (
              <button key={ex} type="button" className={s.example} onClick={() => void send(ex)}>
                {ex}
              </button>
            ))}
          </div>
        )}
        {shown.map((m) =>
          m.role === "user" ? (
            <div key={m.id} className={s.mine}>
              {m.content}
            </div>
          ) : m.role === "assistant" ? (
            <div key={m.id} className={s.theirs}>
              <Rich text={m.content} open={open} />
              <p className="fieldHint aiMark">
                <SparkleIcon /> {m.model}
              </p>
            </div>
          ) : m.proposal ? (
            <Proposal key={m.id} m={m} busy={act.isPending} onAct={(action) => act.mutate({ messageId: m.id, action }, { onError: (e: Error) => setTrouble(e.message) })} open={open} />
          ) : (
            <p key={m.id} className={s.tool}>
              {lookedUp(m)}
            </p>
          ),
        )}
        {pending && (
          <>
            {/* The question shows once: from the thread when it's already kept, else here. */}
            {shown[shown.length - 1]?.content !== pending.text && <div className={s.mine}>{pending.text}</div>}
            <AiProgress label="Thinking" fraction={null} startedAt={pending.at} etaMs={null} name="The assistant is answering" />
          </>
        )}
        <div ref={end} />
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      <div className={s.compose}>
        <textarea
          ref={input}
          className="input"
          rows={2}
          aria-label="Your question"
          placeholder={on ? `Ask about ${on.label || "this record"}…` : "Ask about your CRM…"}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
              e.preventDefault();
              void send();
            }
          }}
        />
        <button type="button" className="btn btnPrimary btnAi btnCompact" disabled={!text.trim() || !!pending} aria-busy={!!pending} onClick={() => void send()}>
          <SparkleIcon /> Ask
        </button>
      </div>
    </aside>
  );
}
