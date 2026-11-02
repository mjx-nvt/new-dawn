import { useState } from "react";
import { useApiKeys, useCreateApiKey, useRevokeApiKey, useSaveWebhook, useWebhooks, type NewApiKey } from "../../kernel/bridge/api";
import DangerButton from "../../kernel/shell/DangerButton";
import { formatWhen } from "../meetings/format";
import { LocalApiGroup, SecretCard, WebhookRow } from "./DevelopersPanel";
import s from "./DevelopersPanel.module.css";

/** Plane's token expiries. */
const EXPIRY: [string, number | null][] = [
  ["1 week", 7],
  ["1 month", 30],
  ["3 months", 90],
  ["1 year", 365],
  ["Never", null],
];

/** Plane's webhook events, by its names. */
const EVENTS: [string, string][] = [
  ["project", "Projects"],
  ["issue", "Work items"],
  ["cycle", "Cycles"],
  ["module", "Modules"],
  ["issue_comment", "Work item comments"],
];

/** A webhook's events in words: "All events", or the ones it wants. */
function eventWords(events: string[]): string {
  if (events.includes("*") || EVENTS.every(([key]) => events.includes(key))) return "All events";
  return EVENTS.filter(([key]) => events.includes(key))
    .map(([, label]) => label)
    .join(", ");
}

/** Workspace → Projects → Developers (D-084): Plane's API on this Mac — the local API's switch,
 *  API tokens (a title, a description, an expiry; shown once) and Plane's webhooks. */
export default function PlaneDevelopersPanel({ slug }: { slug: string }) {
  const keys = useApiKeys(slug);
  const createKey = useCreateApiKey(slug);
  const revoke = useRevokeApiKey(slug);
  const hooks = useWebhooks(slug, "plane");
  const saveHook = useSaveWebhook(slug);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [expiry, setExpiry] = useState(1);
  const [made, setMade] = useState<NewApiKey | null>(null);
  const [url, setUrl] = useState("");
  const [all, setAll] = useState(true);
  const [picked, setPicked] = useState<string[]>(["issue"]);
  const [trouble, setTrouble] = useState<string | null>(null);
  const tokens = (keys.data ?? []).filter((k) => !k.revoked_at);
  const expired = (when: string | null) => !!when && new Date(when) < new Date();
  return (
    <section className="section">
      <h2 className="sectionTitle">Developers</h2>
      <p className="sectionSubtitle">Plane's API on this Mac only — for scripts and tools you run here. Every request needs a token.</p>
      <LocalApiGroup onError={setTrouble} />

      <h3 className="groupTitle">API tokens</h3>
      <div className="group">
        {tokens.map((k) => (
          <div key={k.id} className="groupRow">
            <div className="rowText">
              <p className="rowLabel">
                {k.name} {expired(k.expires_at) && <span className="chip">Expired</span>}
              </p>
              <p className="rowHint">
                {k.description ? `${k.description} · ` : ""}
                <code className={s.code}>{k.prefix}…</code> · {k.expires_at ? `${expired(k.expires_at) ? "expired" : "expires"} ${formatWhen(k.expires_at)}` : "never expires"} · {k.last_used_at ? `last used ${formatWhen(k.last_used_at)}` : "never used"}
              </p>
            </div>
            <div className="rowControl">
              <DangerButton confirm="Revoke this token? Tools using it stop at once." disabled={revoke.isPending} onConfirm={() => revoke.mutate(k.id)}>
                Revoke
              </DangerButton>
            </div>
          </div>
        ))}
        {!tokens.length && (
          <div className="groupRow ghostCard">
            <span className="ghostText">{keys.isPending ? "Reading…" : "No token yet"}</span>
          </div>
        )}
        <div className="groupRow">
          <div className={s.form}>
            <input className="input" aria-label="Token title" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
            <input className="input" aria-label="Token description" placeholder="Description — what it's for" value={description} onChange={(e) => setDescription(e.target.value)} />
            <select className="input" aria-label="Expires" value={expiry} onChange={(e) => setExpiry(Number(e.target.value))}>
              {EXPIRY.map(([label], i) => (
                <option key={label} value={i}>
                  {label === "Never" ? "Never expires" : `Expires in ${label}`}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={!title.trim() || createKey.isPending}
              onClick={() =>
                createKey.mutate(
                  { name: title, description, expiresDays: EXPIRY[expiry]?.[1] ?? null },
                  {
                    onSuccess: (k) => {
                      setMade(k);
                      setTitle("");
                      setDescription("");
                      setTrouble(null);
                    },
                    onError: (e: Error) => setTrouble(e.message),
                  },
                )
              }
            >
              Add token
            </button>
          </div>
        </div>
      </div>
      {made && <SecretCard made={made} onDone={() => setMade(null)} />}
      <p className="fieldHint">Send it as X-API-Key. A token opens this workspace's local API — Plane's paths and Twenty's; Workspace → CRM lists the same keys.</p>

      <h3 className="groupTitle">Webhooks</h3>
      <div className="group">
        {(hooks.data ?? []).map((h) => (
          <WebhookRow key={h.id} slug={slug} hook={h} words={eventWords(h.events)} />
        ))}
        {!hooks.data?.length && (
          <div className="groupRow ghostCard">
            <span className="ghostText">{hooks.isPending ? "Reading…" : "No webhook yet"}</span>
          </div>
        )}
        <div className="groupRow">
          <div className={s.form}>
            <input className="input" aria-label="Webhook address" placeholder="http://localhost:3000/plane" value={url} onChange={(e) => setUrl(e.target.value)} />
            <div className="segmented" role="radiogroup" aria-label="Events">
              <button type="button" role="radio" aria-checked={all} onClick={() => setAll(true)}>
                All events
              </button>
              <button type="button" role="radio" aria-checked={!all} onClick={() => setAll(false)}>
                Individual events
              </button>
            </div>
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={!url.trim() || (!all && !picked.length) || saveHook.isPending}
              onClick={() =>
                saveHook.mutate(
                  { id: null, url, description: "", format: "plane", events: all ? [] : picked, is_active: true },
                  {
                    onSuccess: () => {
                      setUrl("");
                      setTrouble(null);
                    },
                    onError: (e: Error) => setTrouble(e.message),
                  },
                )
              }
            >
              Add webhook
            </button>
          </div>
          {!all && (
            <div className={s.events}>
              {EVENTS.map(([key, label]) => (
                <label key={key} className={s.event}>
                  <input type="checkbox" checked={picked.includes(key)} onChange={(e) => setPicked(e.target.checked ? [...picked, key] : picked.filter((p) => p !== key))} /> {label}
                </label>
              ))}
            </div>
          )}
        </div>
      </div>
      <p className="fieldHint">An address on this Mac only. Each change is posted as Plane's {"{ event, action, webhook_id, workspace_id, data, activity }"}, signed with the webhook's secret (X-Plane-Signature).</p>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
    </section>
  );
}
