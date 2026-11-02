import { useState } from "react";
import { openApiDocs, useApiKeys, useApiStatus, useCreateApiKey, useDeleteWebhook, useRegenerateWebhookSecret, useRevokeApiKey, useSaveWebhook, useSetApiEnabled, useWebhookDeliveries, useWebhooks, type NewApiKey, type Webhook } from "../../kernel/bridge/api";
import DangerButton from "../../kernel/shell/DangerButton";
import { formatWhen } from "../meetings/format";
import OnOff from "./OnOff";
import s from "./DevelopersPanel.module.css";

const EXPIRY: [string, number | null][] = [
  ["Never", null],
  ["30 days", 30],
  ["90 days", 90],
  ["1 year", 365],
];

export function copy(text: string) {
  void navigator.clipboard.writeText(text).catch(() => undefined);
}

/** A webhook's row: its address, events (or `words` for them), on/off, secret — a new one for
 *  Plane's —, deliveries, delete. */
export function WebhookRow({ slug, hook, words }: { slug: string; hook: Webhook; words?: string }) {
  const save = useSaveWebhook(slug);
  const remove = useDeleteWebhook(slug);
  const renew = useRegenerateWebhookSecret(slug);
  const [open, setOpen] = useState(false);
  const deliveries = useWebhookDeliveries(slug, open ? hook.id : null);
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{hook.url}</p>
        <p className="rowHint">
          {words ?? hook.events.join(", ")}
          {hook.description ? ` · ${hook.description}` : ""}
        </p>
        {open && (
          <div className={s.deliveries}>
            <p className="fieldHint">
              Secret <code className={s.code}>{hook.secret}</code>{" "}
              <button type="button" className="btn btnGhost btnCompact" onClick={() => copy(hook.secret)}>
                Copy
              </button>
              {hook.format === "plane" && (
                <DangerButton confirm="The old secret stops at once — new secret?" disabled={renew.isPending} onConfirm={() => renew.mutate(hook.id)}>
                  New secret
                </DangerButton>
              )}
            </p>
            {(deliveries.data ?? []).map((d) => (
              <p key={d.id} className={s.delivery}>
                <span className={d.ok ? `chip ${s.ok}` : `chip ${s.bad}`}>{d.status_code ?? "—"}</span> {d.event} · {formatWhen(d.created_at)} · {d.duration_ms} ms{d.error ? ` — ${d.error}` : ""}
              </p>
            ))}
            {!deliveries.data?.length && <p className="ghostText">No delivery yet</p>}
          </div>
        )}
      </div>
      <div className="rowControl">
        <OnOff label={`${hook.url} on`} value={hook.is_active} disabled={save.isPending} onChange={(on) => save.mutate({ id: hook.id, url: hook.url, description: hook.description, format: hook.format, events: hook.events, is_active: on })} />
        <button type="button" className="btn btnSecondary btnCompact" aria-expanded={open} onClick={() => setOpen(!open)}>
          {open ? "Hide" : "Deliveries"}
        </button>
        <DangerButton confirm="Delete this webhook?" disabled={remove.isPending} onConfirm={() => remove.mutate(hook.id)}>
          Delete
        </DangerButton>
      </div>
    </div>
  );
}

/** The local API's switch, its address and docs — one server for Twenty's paths and Plane's. */
export function LocalApiGroup({ onError }: { onError: (message: string) => void }) {
  const status = useApiStatus();
  const setEnabled = useSetApiEnabled();
  const api = status.data;
  return (
    <div className="group">
      <div className="groupRow">
        <div className="rowText">
          <p className="rowLabel">Local API</p>
          <p className="rowHint">{api?.running ? api.address : api?.enabled ? "On, but not listening — turn it off and on again" : "Off — nothing listens"}</p>
        </div>
        <div className="rowControl">
          {api?.running && (
            <>
              <button type="button" className="btn btnGhost btnCompact" onClick={() => copy(api.address)}>
                Copy address
              </button>
              <button type="button" className="btn btnSecondary btnCompact" onClick={() => void openApiDocs(api.address).catch((e: Error) => onError(e.message))}>
                Open the docs
              </button>
            </>
          )}
          <OnOff label="Local API" value={api?.enabled} disabled={!api || setEnabled.isPending} onChange={(on) => setEnabled.mutate(on, { onError: (e: Error) => onError(e.message) })} />
        </div>
      </div>
    </div>
  );
}

/** A key just made: shown this once, to copy. */
export function SecretCard({ made, onDone }: { made: NewApiKey; onDone: () => void }) {
  return (
    <div className={`card ${s.secret}`} role="status">
      <p className="eyebrow">Shown this once</p>
      <code className={s.code}>{made.secret}</code>
      <div className={s.inline}>
        <button type="button" className="btn btnPrimary btnCompact" onClick={() => copy(made.secret)}>
          Copy the key
        </button>
        <button type="button" className="btn btnGhost btnCompact" onClick={onDone}>
          Done
        </button>
      </div>
      <p className="fieldHint">New Dawn keeps only its fingerprint: copy it now into the tool that uses it.</p>
    </div>
  );
}

/** Workspace → CRM → Developers (D-071): the local API — Twenty's REST API on this Mac, off
 *  until turned on — its keys (shown once), and webhooks to addresses on this Mac. */
export default function DevelopersPanel({ slug }: { slug: string }) {
  const keys = useApiKeys(slug);
  const createKey = useCreateApiKey(slug);
  const revoke = useRevokeApiKey(slug);
  const hooks = useWebhooks(slug);
  const saveHook = useSaveWebhook(slug);
  const [keyName, setKeyName] = useState("");
  const [expiry, setExpiry] = useState(0);
  const [made, setMade] = useState<NewApiKey | null>(null);
  const [url, setUrl] = useState("");
  const [events, setEvents] = useState("*.*");
  const [trouble, setTrouble] = useState<string | null>(null);
  return (
    <section className="section">
      <h2 className="sectionTitle">Developers</h2>
      <p className="sectionSubtitle">Twenty's REST API on this Mac only — for scripts and tools you run here. Every request needs a key.</p>
      <LocalApiGroup onError={setTrouble} />

      <h3 className="groupTitle">API keys</h3>
      <div className="group">
        {(keys.data ?? []).map((k) => (
          <div key={k.id} className="groupRow">
            <div className="rowText">
              <p className="rowLabel">
                {k.name} {k.revoked_at && <span className="chip">Revoked</span>}
              </p>
              <p className="rowHint">
                {k.description ? `${k.description} · ` : ""}
                <code className={s.code}>{k.prefix}…</code> · made {formatWhen(k.created_at)} · {k.expires_at ? `expires ${formatWhen(k.expires_at)}` : "never expires"} · {k.last_used_at ? `last used ${formatWhen(k.last_used_at)}` : "never used"}
              </p>
            </div>
            <div className="rowControl">
              {!k.revoked_at && (
                <DangerButton confirm="Revoke this key?" disabled={revoke.isPending} onConfirm={() => revoke.mutate(k.id)}>
                  Revoke
                </DangerButton>
              )}
            </div>
          </div>
        ))}
        {!keys.data?.length && (
          <div className="groupRow ghostCard">
            <span className="ghostText">{keys.isPending ? "Reading…" : "No key yet"}</span>
          </div>
        )}
        <div className="groupRow">
          <div className={s.form}>
            <input className="input" aria-label="Key name" placeholder="What it's for — a script, a tool" value={keyName} onChange={(e) => setKeyName(e.target.value)} />
            <select className="input" aria-label="Expires" value={expiry} onChange={(e) => setExpiry(Number(e.target.value))}>
              {EXPIRY.map(([label], i) => (
                <option key={label} value={i}>
                  {label}
                </option>
              ))}
            </select>
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={!keyName.trim() || createKey.isPending}
              onClick={() =>
                createKey.mutate(
                  { name: keyName, expiresDays: EXPIRY[expiry]?.[1] ?? null },
                  {
                    onSuccess: (k) => {
                      setMade(k);
                      setKeyName("");
                    },
                    onError: (e: Error) => setTrouble(e.message),
                  },
                )
              }
            >
              New key
            </button>
          </div>
        </div>
      </div>
      {made && <SecretCard made={made} onDone={() => setMade(null)} />}

      <h3 className="groupTitle">Webhooks</h3>
      <div className="group">
        {(hooks.data ?? []).map((h) => (
          <WebhookRow key={h.id} slug={slug} hook={h} />
        ))}
        {!hooks.data?.length && (
          <div className="groupRow ghostCard">
            <span className="ghostText">{hooks.isPending ? "Reading…" : "No webhook yet"}</span>
          </div>
        )}
        <div className="groupRow">
          <div className={s.form}>
            <input className="input" aria-label="Webhook address" placeholder="http://localhost:3000/hook" value={url} onChange={(e) => setUrl(e.target.value)} />
            <input className="input" aria-label="Events" placeholder="*.*, person.created, opportunity.*" value={events} onChange={(e) => setEvents(e.target.value)} />
            <button
              type="button"
              className="btn btnSecondary btnCompact"
              disabled={!url.trim() || saveHook.isPending}
              onClick={() =>
                saveHook.mutate(
                  { id: null, url, description: "", format: "twenty", events: events.split(",").map((e) => e.trim()).filter(Boolean), is_active: true },
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
        </div>
      </div>
      <p className="fieldHint">An address on this Mac only. Each record made, changed or deleted is posted as {"{ event, data, timestamp }"}, signed with the webhook's secret (X-Twenty-Webhook-Signature).</p>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
    </section>
  );
}
