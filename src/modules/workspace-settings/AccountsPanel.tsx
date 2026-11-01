import { useState } from "react";
import { useBlockHandle, useBlocklist, useMailRunning, useMailSettings, useSetMailSettings, useUnblockHandle, type MailSettings } from "../../kernel/bridge/mailbox";
import { useCurrentWorkspace } from "../../kernel/bridge/workspaces";
import styles from "./Settings.module.css";

/** Settings → Accounts (D-066, Twenty's Accounts): Apple Mail as the connected account — its
 *  state and what is read — Contact auto-creation, email visibility and the Blocklist, for
 *  the current workspace. */
export default function AccountsPanel() {
  const { data: workspace } = useCurrentWorkspace();
  const slug = workspace?.slug;
  const running = useMailRunning();
  const settings = useMailSettings(slug);
  const setSettings = useSetMailSettings(slug);
  const blocklist = useBlocklist(slug);
  const block = useBlockHandle(slug);
  const unblock = useUnblockHandle(slug);
  const [handle, setHandle] = useState("");
  const [trouble, setTrouble] = useState<string | null>(null);
  const s = settings.data;
  const save = (patch: Partial<MailSettings>) => {
    if (!s) return;
    setTrouble(null);
    setSettings.mutate({ ...s, ...patch }, { onError: (e: Error) => setTrouble(e.message) });
  };
  const addHandle = () => {
    const h = handle.trim();
    if (!h) return;
    setTrouble(null);
    block.mutate(h, { onSuccess: () => setHandle(""), onError: (e: Error) => setTrouble(e.message) });
  };
  const blocked = blocklist.data ?? [];

  return (
    <>
      <p className="groupTitle">Apple Mail</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Connected account</p>
            <p className="rowHint">
              Mail on this Mac answers for a record's addresses when its Emails tab opens: the inbox from them, your sent messages to them. New Dawn never launches Mail, fetches nothing and sends nothing; the first read asks for your permission (System Settings → Privacy &amp; Security → Automation).
            </p>
          </div>
          <div className="rowControl">
            <span className="chip">{running.data === undefined ? "…" : running.data ? "Mail is running" : "Mail is not running"}</span>
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Contact auto-creation</p>
            <p className="rowHint">Whom a thread proposes as a new person: nobody, anyone you wrote to or who wrote to you, or only those you wrote to.</p>
          </div>
          <div className="rowControl">
            <select className="input" aria-label="Contact auto-creation" value={s?.auto_create ?? "sent_received"} disabled={!s || setSettings.isPending} onChange={(e) => save({ auto_create: e.target.value })}>
              <option value="deactivated">Deactivated</option>
              <option value="sent_received">Sent &amp; Received</option>
              <option value="sent_only">Sent only</option>
            </select>
          </div>
        </div>
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Email visibility</p>
            <p className="rowHint">What is read of a message: who and when only; the subject too; or its first words as well.</p>
          </div>
          <div className="rowControl">
            <select className="input" aria-label="Email visibility" value={s?.visibility ?? "subject"} disabled={!s || setSettings.isPending} onChange={(e) => save({ visibility: e.target.value })}>
              <option value="metadata">Metadata only</option>
              <option value="subject">Subject + metadata</option>
              <option value="everything">Share everything</option>
            </select>
          </div>
        </div>
      </div>

      <p className="groupTitle">Blocklist</p>
      <div className="group">
        {blocked.map((b) => (
          <div key={b.id} className="groupRow">
            <div className="rowText">
              <p className="rowLabel">{b.handle}</p>
              <p className="rowHint">{b.handle.startsWith("@") ? "Everyone at this domain" : "This address"} — never read from Mail, never proposed as a person.</p>
            </div>
            <div className="rowControl">
              <button type="button" className="btn btnGhost btnCompact" disabled={unblock.isPending} onClick={() => unblock.mutate(b.id, { onError: (e: Error) => setTrouble(e.message) })}>
                Remove
              </button>
            </div>
          </div>
        ))}
        {blocked.length === 0 && (
          <div className="groupRow">
            <span className="ghostText">Nobody blocked</span>
          </div>
        )}
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">Add to blocklist</p>
            <p className="rowHint">An address (name@example.com) or a whole domain (@example.com).</p>
          </div>
          <div className="rowControl">
            <input
              className={`input ${styles.rowInput}`}
              aria-label="Address or domain to block"
              placeholder="name@example.com"
              value={handle}
              disabled={!slug || block.isPending}
              onChange={(e) => setHandle(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && !e.nativeEvent.isComposing && addHandle()}
            />
            <button type="button" className="btn btnSecondary btnCompact" disabled={!handle.trim() || block.isPending} onClick={addHandle}>
              Add
            </button>
          </div>
        </div>
      </div>
      {trouble && (
        <p className="formError" role="alert">
          {trouble}
        </p>
      )}
      <p className="fieldHint">These are the settings of the {workspace?.name ?? "current"} workspace.</p>
    </>
  );
}
