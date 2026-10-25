import { useMutation } from "@tanstack/react-query";
import { useEffect, useState, type KeyboardEvent, type ReactNode } from "react";
import { useProfile, useSetProfile } from "../../kernel/bridge/profile";
import { useCreateWorkspace, useUpdateWorkspace, useWorkspaces } from "../../kernel/bridge/workspaces";
import settings from "../workspace-settings/Settings.module.css";
import { suggestSlug } from "../workspace-settings/slug";
import type { StepFrame } from "./TenantSetupPage";

/** Step 1: the workspace's name and folder, and your title in it. Continue creates the workspace
 *  (once) and saves the rest; coming back to this step renames it — the folder stays. */
export default function IdentityStep({ slug, frame, onDone }: { slug?: string; frame: StepFrame; onDone: (slug: string) => void }) {
  const workspaces = useWorkspaces();
  const profile = useProfile().data;
  const create = useCreateWorkspace();
  const update = useUpdateWorkspace();
  const setProfile = useSetProfile();
  const saved = slug ? workspaces.data?.find((w) => w.slug === slug) : undefined;
  // Made by a Continue whose later saves failed: the next try only saves.
  const [created, setCreated] = useState<string | null>(null);
  const made = saved?.slug ?? created;

  const [name, setName] = useState(saved?.name ?? "");
  const [title, setTitle] = useState(saved?.owner_title ?? "");
  useEffect(() => setName(saved?.name ?? ""), [saved?.name]);
  useEffect(() => setTitle(saved?.owner_title ?? ""), [saved?.owner_title]);
  const [slugDraft, setSlugDraft] = useState<string | null>(null); // null: follows the name
  const folder = made ?? slugDraft ?? suggestSlug(name);
  const [you, setYou] = useState("");
  const askName = !!profile && !profile.name;

  const next = useMutation({
    mutationFn: async () => {
      const target = folder; // the registry keeps the slug exactly as given
      if (!made) {
        await create.mutateAsync({ name, slug: target });
        setCreated(target);
      }
      await update.mutateAsync({ slug: target, patch: { owner_title: title.trim(), name: made ? name.trim() : null } });
      if (askName && you.trim()) await setProfile.mutateAsync({ name: you.trim() });
      return target;
    },
  });
  const disabled = !name.trim() || (!!slug && workspaces.isPending);
  const run = () => next.mutate(undefined, { onSuccess: onDone });
  const onEnter = (e: KeyboardEvent<HTMLInputElement>) => e.key === "Enter" && !e.nativeEvent.isComposing && !disabled && !next.isPending && run();

  return frame(
    <>
      <p className="fieldHint">Each company you work with gets its own workspace: its own meetings, brand and minutes template.</p>

      <p className="groupTitle">Workspace</p>
      <div className="group">
        <Row label="Name" hint="Shown in the sidebar and on documents as the organisation.">
          <input className={`input ${settings.rowInput}`} value={name} maxLength={100} aria-label="Name" autoFocus={!made} onChange={(e) => setName(e.target.value)} onKeyDown={onEnter} />
        </Row>
        <Row label="Slug" hint={made ? `Its folder: workspaces/${made}/` : `Its folder: workspaces/${folder || "…"}/ — fixed once the workspace exists.`}>
          {made ? (
            <span className="chip">{made}</span>
          ) : (
            <input className={`input ${settings.rowInput}`} value={folder} maxLength={40} aria-label="Slug" spellCheck={false} onChange={(e) => setSlugDraft(e.target.value)} onKeyDown={onEnter} />
          )}
        </Row>
        {askName ? (
          <Row label="Your name" hint="Saved in Settings → General — the same in every workspace.">
            <input className={`input ${settings.rowInput}`} value={you} maxLength={120} placeholder="Not yet captured" aria-label="Your name" onChange={(e) => setYou(e.target.value)} onKeyDown={onEnter} />
          </Row>
        ) : (
          <Row label="You" hint="Your name, from Settings → General — the same in every workspace.">
            <span className="fieldValue">{profile?.name}</span>
          </Row>
        )}
        <Row label="Your title here" hint="Optional. With your name on the minutes and in your email sign-off.">
          <input className={`input ${settings.rowInput}`} value={title} maxLength={100} placeholder="Not yet captured" aria-label="Your title here" onChange={(e) => setTitle(e.target.value)} onKeyDown={onEnter} />
        </Row>
      </div>
    </>,
    { label: next.isPending ? "Saving…" : made ? "Continue" : "Create workspace", run, pending: next.isPending, disabled, error: next.error },
  );
}

function Row({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return (
    <div className="groupRow">
      <div className="rowText">
        <p className="rowLabel">{label}</p>
        <p className="rowHint">{hint}</p>
      </div>
      <div className="rowControl">{children}</div>
    </div>
  );
}
