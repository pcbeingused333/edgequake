import type { EligibleModel, ResolvedUsage } from "@/lib/api/edgequake/model-catalog";

/** True when the workspace query default is a pair the chip may run. */
export function inheritIsPossible(
  queryUsage: ResolvedUsage | undefined,
  eligible: EligibleModel[],
): boolean {
  if (!queryUsage) return false;
  if (queryUsage.broken_reason !== "none") return false;
  if (queryUsage.eligible === false) return false;
  const provider = queryUsage.provider?.trim();
  const model = queryUsage.model?.trim();
  if (!provider || !model) return false;
  return eligible.some((entry) => entry.provider === provider && entry.model === model);
}

/** True when Send must wait for an explicit eligible pick (Inherit cannot run). */
export function choiceSendBlocked(input: {
  eligibleLoaded: boolean;
  eligibleEmpty: boolean;
  inheritOk: boolean;
  overrideProvider?: string;
  overrideModel?: string;
}): boolean {
  if (!input.eligibleLoaded) return false;
  if (input.eligibleEmpty) return true;
  const hasOverride = Boolean(input.overrideProvider?.trim() && input.overrideModel?.trim());
  if (hasOverride) return false;
  return !input.inheritOk;
}
