import type { LlmModelItem } from "@/lib/api/models";

/** Fleet checklist refresh: probe must already be ok. */
export function showFleetModelRefresh(refreshable: boolean, probeOk: boolean): boolean {
  return refreshable && probeOk;
}

/**
 * Deployment refresh. Env slugs refresh when the probe is ok or unset.
 * Fleet slugs refresh only after a successful probe. Neither path is offered
 * when the server marks the slug not refreshable.
 */
export function showProviderModelRefresh(input: {
  refreshable: boolean;
  origin: string;
  probe?: string | null;
}): boolean {
  if (!input.refreshable) return false;
  const probe = input.probe?.trim() ?? "";
  if (input.origin === "env") return probe === "" || probe === "ok";
  if (input.origin === "fleet") return probe === "ok";
  return false;
}

export function connectionModelBody(
  ids: string[],
  discovered: LlmModelItem[],
): { id: string; supports_chat: boolean; supports_vision: boolean; supports_embedding: boolean }[] {
  return ids.map((id) => {
    const live = discovered.find((item) => item.name === id);
    return {
      id,
      supports_chat: true,
      supports_vision: live?.capabilities.supports_vision === true,
      supports_embedding: false,
    };
  });
}

export function toggleModelId(ids: string[], id: string, checked: boolean): string[] {
  if (checked) return ids.includes(id) ? ids : [...ids, id];
  return ids.filter((current) => current !== id);
}
