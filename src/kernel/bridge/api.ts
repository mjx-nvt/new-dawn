import { openUrl } from "@tauri-apps/plugin-opener";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { commands, type ApiKey_Serialize, type ApiStatus, type Delivery_Serialize, type NewApiKey_Serialize, type Webhook, type WebhookInput_Deserialize } from "../../bindings";
import { unwrap } from "./unwrap";

/** The local API (D-071, 0.10.14): on/off and its address, keys, webhooks. */
export type { ApiStatus, Webhook };
export type ApiKey = ApiKey_Serialize;
export type NewApiKey = NewApiKey_Serialize;
export type Delivery = Delivery_Serialize;
export type WebhookInput = WebhookInput_Deserialize;

export function useApiStatus() {
  return useQuery({ queryKey: ["api-status"], queryFn: () => unwrap(commands.apiStatus()) });
}

export function useSetApiEnabled() {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (on: boolean) => unwrap(commands.setApiEnabled(on)), onSuccess: (status) => queryClient.setQueryData(["api-status"], status) });
}

export function useApiKeys(slug: string | undefined) {
  return useQuery({ queryKey: ["api-keys", slug ?? ""], queryFn: () => unwrap(commands.listApiKeys(slug ?? "")), enabled: !!slug });
}

export function useCreateApiKey(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ name, expiresDays }: { name: string; expiresDays: number | null }) => unwrap(commands.createApiKey(slug ?? "", name, expiresDays)),
    onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["api-keys", slug ?? ""] }),
  });
}

export function useRevokeApiKey(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (id: string) => unwrap(commands.revokeApiKey(slug ?? "", id)), onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["api-keys", slug ?? ""] }) });
}

export function useWebhooks(slug: string | undefined) {
  return useQuery({ queryKey: ["webhooks", slug ?? ""], queryFn: () => unwrap(commands.listWebhooks(slug ?? "")), enabled: !!slug });
}

export function useSaveWebhook(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (input: WebhookInput) => unwrap(commands.saveWebhook(slug ?? "", input)), onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["webhooks", slug ?? ""] }) });
}

export function useDeleteWebhook(slug: string | undefined) {
  const queryClient = useQueryClient();
  return useMutation({ mutationFn: (id: string) => unwrap(commands.deleteWebhook(slug ?? "", id)), onSuccess: () => void queryClient.invalidateQueries({ queryKey: ["webhooks", slug ?? ""] }) });
}

/** A webhook's last deliveries, read again every few seconds while shown. */
export function useWebhookDeliveries(slug: string | undefined, webhookId: string | null) {
  return useQuery({ queryKey: ["webhook-deliveries", slug ?? "", webhookId ?? ""], queryFn: () => unwrap(commands.webhookDeliveries(slug ?? "", webhookId ?? "")), enabled: !!slug && !!webhookId, refetchInterval: 5000 });
}

/** The docs page of the running API, in the browser (an address on this Mac). */
export function openApiDocs(address: string): Promise<void> {
  if (!/^http:\/\/localhost:\d+$/.test(address)) return Promise.reject(new Error("The local API isn't running"));
  return openUrl(`${address}/docs`);
}
