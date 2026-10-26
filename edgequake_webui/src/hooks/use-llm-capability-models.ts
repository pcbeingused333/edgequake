/**
 * Capability lookup for LLM models (thinking, vision, streaming).
 *
 * Capability lookups only. Picker lists come from the eligible endpoint, so this hook never
 * decides what a user can choose; it answers "what does this model support?" for a pair that
 * is already selected.
 */
import { useMemo } from "react";
import { useModelsConfig } from "@/hooks/use-models";
import type { ModelResponse, ModelsListResponse } from "@/lib/api/models";

/**
 * Nested cards on `GET /models` do not include `provider`.
 * Capability lookups match on provider + name, so copy the parent identity onto each card.
 */
export function llmCapabilityModelsFromConfig(
  data: ModelsListResponse | undefined,
): ModelResponse[] | undefined {
  if (!data?.providers) return undefined;
  return data.providers.flatMap((provider) =>
    (provider.models ?? []).map((model) => ({
      ...model,
      provider: model.provider || provider.name,
      provider_display_name:
        model.provider_display_name || provider.display_name || provider.name,
    })),
  );
}

export function useLlmCapabilityModels(): ModelResponse[] | undefined {
  const { data } = useModelsConfig();
  return useMemo(() => llmCapabilityModelsFromConfig(data), [data]);
}
