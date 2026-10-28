/**
 * SPEC-175 — the caller's settings access, as the server reports it.
 *
 * The server is the authority (LAW-175-13). These predicates only decide what the
 * shell offers. `GET /api/v1/me/settings-access` supplies the raw flags; this module
 * turns them into the badge (LAW-175-17) and the gates in `registry.ts`.
 */

export interface SettingsAccessPayload {
  user_id: string;
  /** Platform admin: `edgequake_auth::Role::Admin`. */
  instance_admin: boolean;
  /** Tenant-wide owner or admin row (`applies_to` without a workspace id). */
  tenant_role: "owner" | "admin" | "member" | "readonly" | null;
  /** Super admin rights in this tenant (`TenantAccess::Manage`). */
  tenant_manage: boolean;
  /** Selected workspace, when one was requested. */
  workspace_id: string | null;
  /** Owner or admin of the requested workspace (`can_manage_workspaces`). */
  workspace_manage: boolean;
  /** Workspaces this caller may administer in the tenant. */
  administered_workspace_ids: string[];
}

export interface SettingsAccess {
  instanceAdmin: boolean;
  tenantManage: boolean;
  workspaceManage: boolean;
  administeredWorkspaceIds: readonly string[];
}

export type AccessChip = "platform_admin" | "super_admin" | "workspace_admin" | "member";

/** Empty grant. Used while the access request is in flight: nothing gated is offered. */
export const NO_SETTINGS_ACCESS: SettingsAccess = {
  instanceAdmin: false,
  tenantManage: false,
  workspaceManage: false,
  administeredWorkspaceIds: [],
};

export function toSettingsAccess(payload: SettingsAccessPayload): SettingsAccess {
  const ids = payload.administered_workspace_ids;
  const instanceAdmin = payload.instance_admin === true;
  const tenantManage = payload.tenant_manage === true || instanceAdmin;
  return {
    instanceAdmin,
    tenantManage,
    workspaceManage: payload.workspace_manage === true || tenantManage,
    administeredWorkspaceIds: Array.isArray(ids) ? ids : [],
  };
}

/**
 * Badge = the highest grant (LAW-175-17): Platform admin, else Super admin, else
 * Workspace admin, else Member. Platform admin never reads as Super admin.
 */
export function accessChip(access: SettingsAccess): AccessChip {
  if (access.instanceAdmin) return "platform_admin";
  if (access.tenantManage) return "super_admin";
  if (access.workspaceManage || (access.administeredWorkspaceIds?.length ?? 0) > 0) return "workspace_admin";
  return "member";
}

export const ACCESS_CHIP_COPY: Record<AccessChip, { label: string; fallback: string }> = {
  platform_admin: { label: "settings.access.platformAdmin", fallback: "Platform admin" },
  super_admin: { label: "settings.access.superAdmin", fallback: "Super admin" },
  workspace_admin: { label: "settings.access.workspaceAdmin", fallback: "Workspace admin" },
  member: { label: "settings.access.member", fallback: "Member" },
};