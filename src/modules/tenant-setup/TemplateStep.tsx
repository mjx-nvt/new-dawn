import { useGenerateTemplate, useMeetingTypeTemplates, useOpenTypeTemplate } from "../../kernel/bridge/templates";
import { useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";
import DangerButton from "../../kernel/shell/DangerButton";
import { MinutesPreview } from "../workspace-settings/BrandPanel";
import { CheckSummary } from "../workspace-settings/MeetingTypesPanel";
import type { StepFrame } from "./TenantSetupPage";

/** Step 3: the minutes template made from the brand, drawn with a made-up meeting. Finish makes
 *  the workspace the current one. */
export default function TemplateStep({ slug, frame, onDone }: { slug: string; frame: StepFrame; onDone: () => void }) {
  const types = useMeetingTypeTemplates(slug);
  const generate = useGenerateTemplate(slug);
  const open = useOpenTypeTemplate(slug);
  const choose = useSetCurrentWorkspace();
  const main = types.data?.find((t) => t.is_default) ?? types.data?.[0];

  return frame(
    <>
      <p className="fieldHint">Made from your brand. Change anything in Word; the meeting, names and slogans fill in when you export.</p>

      <p className="groupTitle">Minutes template</p>
      <div className="group">
        <div className="groupRow">
          <div className="rowText">
            <p className="rowLabel">{main ? `${main.name} (default type)` : "Default meeting type"}</p>
            {main?.template.check ? <CheckSummary check={main.template.check} /> : <p className="rowHint ghostText">Not yet captured</p>}
          </div>
          <div className="rowControl">
            <button type="button" className="btn btnSecondary btnCompact" disabled={!main || open.isPending} onClick={() => main && open.mutate({ id: main.id, reveal: false })}>
              Open in Word
            </button>
            <DangerButton confirm="Make the minutes template again from this brand? The current one goes to the Trash." onConfirm={() => generate.mutate()} disabled={generate.isPending}>
              {generate.isPending ? "Making…" : "Make again"}
            </DangerButton>
          </div>
        </div>
        <MinutesPreview slug={slug} />
      </div>
    </>,
    {
      label: "Finish",
      run: () => choose.mutate(slug, { onSuccess: onDone }),
      pending: choose.isPending,
      error: choose.error ?? generate.error ?? open.error ?? types.error,
    },
  );
}
