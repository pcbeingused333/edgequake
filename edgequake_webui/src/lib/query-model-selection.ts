export interface QueryModelSelection {
  provider?: string;
  model?: string;
}

export interface QueryModelInfo {
  provider: string;
  name: string;
}

export interface QueryProviderHealth {
  name: string;
  enabled?: boolean;
  health?: {
    available?: boolean;
  };
}

/**
 * True when the backend rejected LLM credentials (invalid/missing API key).
 */
export function isLlmProviderAuthFailure(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    lower.includes("invalid_api_key") ||
    lower.includes("incorrect api key") ||
    lower.includes("authentication error") ||
    lower.includes("invalid api key")
  );
}

/**
 * Clears stale persisted query model selections when the provider/model is no longer
 * available in the current runtime environment. This prevents the UI from sticking
 * to a dead local provider such as Ollama when the backend server default is healthy.
 */
/**
 * Drops a persisted pair that the eligible list does not contain.
 * `loaded: false` leaves the pair alone. A loaded empty list clears it.
 */
export function sanitizeQueryModelSelection(
  selection: QueryModelSelection,
  models?: QueryModelInfo[],
  providerHealth?: QueryProviderHealth[],
  loaded = true,
): QueryModelSelection {
  const provider = selection.provider?.trim();
  const model = selection.model?.trim();

  if (!provider && !model) {
    return { provider: undefined, model: undefined };
  }

  if (!provider || !model) {
    return { provider: undefined, model: undefined };
  }

  if (!loaded) {
    return { provider, model };
  }

  if (!models) {
    return { provider, model };
  }

  const modelExists = models.some((entry) => entry.provider === provider && entry.name === model);
  if (!modelExists) {
    return { provider: undefined, model: undefined };
  }

  if (providerHealth?.length) {
    const providerStatus = providerHealth.find((entry) => entry.name === provider);
    const providerEnabled = providerStatus?.enabled ?? false;
    const providerAvailable = providerStatus?.health?.available ?? false;
    if (!providerEnabled || !providerAvailable) {
      return { provider: undefined, model: undefined };
    }
  }

  return { provider, model };
}
