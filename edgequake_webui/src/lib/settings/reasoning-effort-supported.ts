/**
 * SPEC-109: look up catalog-supported reasoning effort values for a model.
 */

import type { ModelResponse } from "@/lib/api/models";

export type ReasoningEffortRolePolicy = "structured" | "query" | "fleet";

function normalizedId(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed.toLowerCase() : undefined;
}

function findModel(
  models: ModelResponse[] | undefined,
  provider?: string | null,
  model?: string | null,
): ModelResponse | undefined {
  const wantProvider = normalizedId(provider);
  const wantModel = normalizedId(model);
  if (!models?.length || !wantProvider || !wantModel) return undefined;
  // Nested `/models` cards omit `provider` until the capability hook stamps the parent.
  // A missing field must not throw — the workspace page renders this lookup unconditionally.
  return models.find((entry) => {
    return (
      normalizedId(entry?.provider) === wantProvider &&
      normalizedId(entry?.name) === wantModel
    );
  });
}

export function modelSupportsThinking(
  models: ModelResponse[] | undefined,
  provider?: string | null,
  model?: string | null,
): boolean | undefined {
  const hit = findModel(models, provider, model);
  if (!hit) return undefined;
  return hit.capabilities?.supports_thinking === true;
}

export function supportedReasoningEffortsForModel(
  models: ModelResponse[] | undefined,
  provider?: string | null,
  model?: string | null,
): string[] | undefined {
  const hit = findModel(models, provider, model);
  // SPEC-113: when catalog explicitly says no thinking, hide effort controls.
  if (hit?.capabilities?.supports_thinking === false) {
    return undefined;
  }
  const supported = hit?.capabilities?.reasoning_effort?.supported;
  if (!supported || supported.length === 0) return undefined;
  return supported;
}

/**
 * Lowest effort recommended for structured JSON roles (extract/summary/keyword/vlm).
 * Falls back to first supported value when catalog omits `lowest_structured`.
 */
export function lowestStructuredEffortForModel(
  models: ModelResponse[] | undefined,
  provider?: string | null,
  model?: string | null,
): string | undefined {
  const hit = findModel(models, provider, model);
  const caps = hit?.capabilities?.reasoning_effort;
  if (!caps) return undefined;
  const lowest = caps.lowest_structured?.trim();
  if (lowest) return lowest;
  return caps.supported?.[0];
}

/**
 * Effective value when the UI choice is Auto (inherit).
 * - structured / fleet: lowest supported (EdgeQuake best practice for JSON roles)
 * - query: omit — do not send the field (provider model default)
 * - no reasoning capability: omit
 */
export function effectiveEffortWhenAuto(
  models: ModelResponse[] | undefined,
  provider: string | null | undefined,
  model: string | null | undefined,
  policy: ReasoningEffortRolePolicy = "structured",
): string {
  const supported = supportedReasoningEffortsForModel(models, provider, model);
  if (!supported || supported.length === 0) {
    return "omit";
  }
  if (policy === "query") {
    return "omit";
  }
  return (
    lowestStructuredEffortForModel(models, provider, model) ??
    supported[0] ??
    "omit"
  );
}

/** Human label for Auto option / helper (best-practice effective). */
export function formatAutoEffortLabel(effective: string): string {
  const e = effective.trim().toLowerCase();
  if (!e || e === "omit") {
    return "Auto (inherit) · effective: omit (provider default)";
  }
  return `Auto (inherit) · effective: ${e} (best practice)`;
}

/** Short helper under the select when Auto is selected. */
export function formatEffectiveBestPracticeHint(effective: string): string {
  const e = effective.trim().toLowerCase();
  if (!e || e === "omit") {
    return "Best practice when Auto: omit field (provider default)";
  }
  return `Best practice when Auto: ${e}`;
}
