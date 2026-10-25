/**
 * SPEC-175 — model policy, eligible models, effective usages and caller access.
 *
 * Mirrors the server contract in `specs/175-perfect-settings/08-api-contract.md`.
 * The UI never recomputes the ladder (LAW-175-1): `ResolvedUsage` comes from the
 * server, member pickers read `/models/eligible`, and tenant managers read
 * `/tenants/{id}/model-catalog`.
 */

import { api } from "../client";
import type { SettingsAccessPayload } from "@/lib/settings/access";

/** The usages a model can be assigned to. `rerank` is never a row (EC-175-09). */
export const USAGE_KEYS = [
  "extract",
  "query",
  "summary",
  "vlm",
  "keyword",
  "embedding",
  "decision",
] as const;
export type UsageKey = (typeof USAGE_KEYS)[number];

export type ResolutionSource =
  | "operator_lock"
  | "request"
  | "workspace_role"
  | "workspace_default"
  | "tenant_default"
  | "server_config"
  | "env_bootstrap"
  | "compiled_default";

export type BrokenReason = "none" | "not_eligible" | "capability" | "credential" | "probe" | "cross_tenant";

export type PolicyMode = "unconstrained" | "constrained";

/** One usage as the server resolved it. Mirrors `EffectiveUsageDto`. */
export interface ResolvedUsage {
  usage: UsageKey;
  provider: string;
  provider_display_name: string | null;
  model: string;
  connection_id: string | null;
  reasoning_effort: string | null;
  source: ResolutionSource;
  locked: boolean;
  eligible: boolean;
  broken_reason: BrokenReason;
}

/** `GET /config/effective?workspace_id=` (the legacy fields are ignored here). */
export interface EffectiveUsagesResponse {
  workspace_id: string | null;
  policy_mode: PolicyMode;
  usages: ResolvedUsage[];
}

/** One selectable model for a workspace and usage. Mirrors `EligibleModelDto`. */
export interface EligibleModel {
  provider: string;
  provider_display_name: string;
  model: string;
  display_name: string;
  supports_vision: boolean;
}

/** `GET /models/eligible`. `policy_mode` is per usage. */
export interface EligibleModelsResponse {
  usage: UsageKey;
  policy_mode: PolicyMode;
  models: EligibleModel[];
}

/** One registered deployment model for the tenant ceiling editor. Mirrors `CatalogEntryDto`. */
export interface CatalogEntry {
  provider: string;
  provider_display_name: string;
  model: string;
  display_name: string;
  health: "healthy" | "probe_failed" | "unknown";
  usages: UsageKey[];
}

/** `GET /tenants/{id}/model-catalog`. */
export interface CatalogResponse {
  policy_mode: PolicyMode;
  entries: CatalogEntry[];
}

/** One policy row. `enabled: false` on a workspace row removes the pair for that usage. */
export interface PolicyRow {
  provider: string;
  model: string;
  usage: UsageKey;
  enabled: boolean;
}

/** `GET|PUT /tenants/{id}/model-policy`. */
export interface TenantPolicyResponse {
  tenant_id: string;
  constrained_usages: UsageKey[];
  rows: PolicyRow[];
}

/** `GET|PUT /workspaces/{id}/model-policy`. `can_manage` decides whether to show the editor. */
export interface WorkspacePolicyResponse {
  workspace_id: string;
  can_manage: boolean;
  rows: PolicyRow[];
}

const tenantPath = (tenantId: string) => `/tenants/${encodeURIComponent(tenantId)}`;
const workspacePath = (workspaceId: string) => `/workspaces/${encodeURIComponent(workspaceId)}`;

export const fetchSettingsAccess = (tenantId: string, workspaceId?: string | null) => {
  const params = new URLSearchParams({ tenant_id: tenantId });
  if (workspaceId) params.set("workspace_id", workspaceId);
  return api.get<SettingsAccessPayload>(`/me/settings-access?${params.toString()}`);
};

export const fetchEffectiveUsages = (workspaceId: string) =>
  api.get<EffectiveUsagesResponse>(`/config/effective?workspace_id=${encodeURIComponent(workspaceId)}`);

export const fetchEligibleModels = (workspaceId: string, usage: UsageKey) =>
  api.get<EligibleModelsResponse>(
    `/models/eligible?workspace_id=${encodeURIComponent(workspaceId)}&usage=${usage}`,
  );

export const fetchTenantModelCatalog = (tenantId: string) =>
  api.get<CatalogResponse>(`${tenantPath(tenantId)}/model-catalog`);

export const getTenantModelPolicy = (tenantId: string) =>
  api.get<TenantPolicyResponse>(`${tenantPath(tenantId)}/model-policy`);

/** Full replace of the ceiling. An empty `rows` array returns the tenant to unconstrained. */
export const putTenantModelPolicy = (tenantId: string, rows: PolicyRow[]) =>
  api.put<TenantPolicyResponse>(`${tenantPath(tenantId)}/model-policy`, { rows });

export const getWorkspaceModelPolicy = (workspaceId: string) =>
  api.get<WorkspacePolicyResponse>(`${workspacePath(workspaceId)}/model-policy`);

/** Full replace of the narrowing rows for one workspace. */
export const putWorkspaceModelPolicy = (workspaceId: string, rows: PolicyRow[]) =>
  api.put<WorkspacePolicyResponse>(`${workspacePath(workspaceId)}/model-policy`, { rows });
