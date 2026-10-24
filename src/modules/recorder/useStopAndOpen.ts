import { useNavigate } from "@tanstack/react-router";
import { useRecorderStatus, useStopRecording } from "../../kernel/bridge/recorder";
import { useCurrentWorkspace, useSetCurrentWorkspace } from "../../kernel/bridge/workspaces";

/** Stop the recording, then open its meeting (transcription is already queued). The meeting
 *  page reads the current workspace, so switch to the recording's workspace first if needed. */
export function useStopAndOpen() {
  const stop = useStopRecording();
  const { data: status } = useRecorderStatus();
  const { data: current } = useCurrentWorkspace();
  const switchWorkspace = useSetCurrentWorkspace();
  const navigate = useNavigate();
  return {
    stop: () => {
      const slug = status?.workspace_slug;
      stop.mutate(undefined, {
        onSuccess: async (meeting) => {
          if (slug && slug !== current?.slug) await switchWorkspace.mutateAsync(slug).catch(() => undefined);
          void navigate({ to: "/meetings/$meetingId", params: { meetingId: meeting.id }, search: { tab: "transcript" } });
        },
      });
    },
    isPending: stop.isPending,
    error: stop.error,
  };
}
