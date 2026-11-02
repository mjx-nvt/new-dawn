import { useState } from "react";
import { ASSISTANT_TOOLS, useAssistantAgents, useDeleteAssistantAgent, useSaveAssistantAgent, type AssistantAgent, type AssistantAgentInput } from "../../kernel/bridge/assistant";
import { useOllamaStatus } from "../../kernel/bridge/summary";
import DangerButton from "../../kernel/shell/DangerButton";
import s from "./AssistantAgentsPanel.module.css";

function blank(): AssistantAgentInput {
  return { id: null, name: "", description: "", instructions: "", model: "", tools: ASSISTANT_TOOLS.filter((t) => !t.writes).map((t) => t.name) };
}

/** One agent's settings: name, description, instructions, model, the tools it may use. */
function Editor({ slug, start, isDefault, onDone }: { slug: string; start: AssistantAgentInput; isDefault: boolean; onDone: () => void }) {
  const [draft, setDraft] = useState<AssistantAgentInput>(start);
  const ollama = useOllamaStatus();
  const save = useSaveAssistantAgent(slug);
  const remove = useDeleteAssistantAgent(slug);
  const tools = draft.tools ?? [];
  const set = (patch: Partial<AssistantAgentInput>) => setDraft({ ...draft, ...patch });
  return (
    <div className={`card ${s.editor}`}>
      <div className="fields">
        <div className="field">
          <label className="fieldLabel" htmlFor="agent-name">
            Name
          </label>
          <input id="agent-name" className="input" value={draft.name} onChange={(e) => set({ name: e.target.value })} />
        </div>
        <div className="field">
          <label className="fieldLabel" htmlFor="agent-model">
            Model
          </label>
          <select id="agent-model" className="input" value={draft.model ?? ""} onChange={(e) => set({ model: e.target.value })}>
            <option value="">App default{ollama.data?.model ? ` (${ollama.data.model})` : ""}</option>
            {(ollama.data?.installed ?? []).map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
              </option>
            ))}
          </select>
        </div>
      </div>
      <div className="field">
        <label className="fieldLabel" htmlFor="agent-description">
          Description
        </label>
        <input id="agent-description" className="input" value={draft.description ?? ""} onChange={(e) => set({ description: e.target.value })} />
      </div>
      <div className="field">
        <label className="fieldLabel" htmlFor="agent-instructions">
          Instructions
        </label>
        <textarea id="agent-instructions" className="input" rows={5} value={draft.instructions ?? ""} placeholder="What it's for, how it answers, what it looks at first." onChange={(e) => set({ instructions: e.target.value })} />
      </div>
      <fieldset className={s.tools}>
        <legend className="fieldLabel">Tools</legend>
        {ASSISTANT_TOOLS.map((t) => (
          <label key={t.name} className={s.tool}>
            <input type="checkbox" checked={tools.includes(t.name)} onChange={(e) => set({ tools: e.target.checked ? [...tools, t.name] : tools.filter((x) => x !== t.name) })} />
            {t.label}
            {t.writes && <span className="fieldHint"> — a proposal you apply</span>}
          </label>
        ))}
      </fieldset>
      {(save.error ?? remove.error) && (
        <p className="formError" role="alert">
          {(save.error ?? remove.error)?.message}
        </p>
      )}
      <div className="sheetActions">
        {draft.id && !isDefault && (
          <DangerButton confirm="Delete this agent?" disabled={remove.isPending} onConfirm={() => draft.id && remove.mutate(draft.id, { onSuccess: onDone })}>
            Delete
          </DangerButton>
        )}
        <button type="button" className="btn btnSecondary" onClick={onDone}>
          Cancel
        </button>
        <button type="button" className="btn btnPrimary" disabled={!draft.name.trim() || save.isPending} onClick={() => save.mutate(draft, { onSuccess: onDone })}>
          Save
        </button>
      </div>
    </div>
  );
}

/** Workspace → CRM → AI agents (D-070): the assistant's agents — the default and the owner's —
 *  each its instructions, its model and the tools it may use. Changes are only ever proposals. */
export default function AssistantAgentsPanel({ slug }: { slug: string }) {
  const agents = useAssistantAgents(slug);
  const [editing, setEditing] = useState<{ start: AssistantAgentInput; isDefault: boolean } | null>(null);
  const edit = (a: AssistantAgent) => setEditing({ start: { id: a.id, name: a.name, description: a.description, instructions: a.instructions, model: a.model, tools: a.tools }, isDefault: a.is_default });
  return (
    <section className="section">
      <h2 className="sectionTitle">AI agents</h2>
      <p className="sectionSubtitle">The assistant answers over this CRM with the local model (⌘K → Ask AI); an agent is its instructions, its model and the tools it may use. Changes are proposals you apply.</p>
      <div className="group">
        {(agents.data ?? []).map((a) => (
          <div key={a.id} className="groupRow">
            <div className="rowText">
              <span className="rowLabel">
                {a.name} {a.is_default && <span className="chip">Default</span>}
              </span>
              <span className="rowHint">{[a.description, a.model || "App default model", `${a.tools.length} tools`].filter(Boolean).join(" · ")}</span>
            </div>
            <div className="rowControl">
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => edit(a)}>
                Edit
              </button>
            </div>
          </div>
        ))}
        {!agents.data?.length && (
          <div className="groupRow ghostCard">
            <span className="ghostText">{agents.isError ? "Couldn't be read" : "Reading…"}</span>
          </div>
        )}
      </div>
      {editing ? (
        <Editor key={editing.start.id ?? "new"} slug={slug} start={editing.start} isDefault={editing.isDefault} onDone={() => setEditing(null)} />
      ) : (
        <div className={s.add}>
          <button type="button" className="btn btnSecondary btnCompact" onClick={() => setEditing({ start: blank(), isDefault: false })}>
            New agent
          </button>
        </div>
      )}
    </section>
  );
}
