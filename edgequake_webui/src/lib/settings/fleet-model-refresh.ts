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
