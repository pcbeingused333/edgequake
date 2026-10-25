"use client";

/**
 * SPEC-175 — everything the settings shell needs to decide what to render.
 *
 * - Access comes from the server (`/me/settings-access`), never from the local role.
 * - The workspace is the header selection, or the slug on `/w/{slug}/settings`.
 * - While access is in flight the shell renders no gated content (fail closed).
 */

import { fetchSettingsAccess } from "@/lib/api/edgequake/model-catalog";
import {
  NO_SETTINGS_ACCESS,
  toSettingsAccess,
  type SettingsAccess,
  type SettingsAccessPayload,
} from "@/lib/settings/access";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { Workspace } from "@/types";
import { useQuery } from "@tanstack/react-query";
import { useWorkspaceSlugResolver } from "@/hooks/use-workspace-slug-resolver";

export type WorkspaceState = "none" | "loading" | "ready" | "missing";

export interface SettingsContext {
  access: SettingsAccess;
  /** False until the server has answered. Gated content stays hidden. */
  accessReady: boolean;
  accessError: boolean;
  tenantId: string | null;
  workspace: Workspace | null;
  workspaceState: WorkspaceState;
  /** Workspaces the caller may administer; drives the settings switcher. */
  administeredWorkspaces: readonly Workspace[];
}

/** Server flags for the caller in this tenant and the selected workspace. */
export function useSettingsAccess(tenantId: string | null, workspaceId: string | null) {
  return useQuery<SettingsAccessPayload>({
    queryKey: ["settings-access", tenantId, workspaceId],
    queryFn: () => fetchSettingsAccess(tenantId as string, workspaceId),
    enabled: !!tenantId,
    staleTime: 30_000,
    retry: false,
  });
}

/**
 * @param workspaceSlug slug from `/w/{slug}/settings`, or null for the header selection.
 */
export function useSettingsContext(workspaceSlug: string | null): SettingsContext {
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const storeWorkspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const workspaces = useTenantStore((s) => s.workspaces);

  const slugResolution = useWorkspaceSlugResolver(workspaceSlug);
  const slugWorkspace = workspaceSlug ? slugResolution.workspace ?? null : null;

  const storeWorkspace = workspaces.find((w) => w.id === storeWorkspaceId) ?? null;
  const workspace = workspaceSlug ? slugWorkspace : storeWorkspace;
  const workspaceId = workspace?.id ?? null;

  const accessQuery = useSettingsAccess(tenantId, workspaceId);
  const access = accessQuery.data ? toSettingsAccess(accessQuery.data) : NO_SETTINGS_ACCESS;

  const administeredIds = new Set(access.administeredWorkspaceIds);
  const administeredWorkspaces = access.tenantManage || access.instanceAdmin
    ? workspaces
    : workspaces.filter((w) => administeredIds.has(w.id));

  let workspaceState: WorkspaceState = "none";
  if (workspaceSlug) {
    if (slugResolution.error) workspaceState = "missing";
    else if (workspace) workspaceState = "ready";
    else workspaceState = "loading";
  } else if (workspace) {
    workspaceState = "ready";
  }

  return {
    access,
    accessReady: !!accessQuery.data,
    accessError: !!accessQuery.error,
    tenantId,
    workspace,
    workspaceState,
    administeredWorkspaces,
  };
}

/**
 * Whether the caller may change the selected workspace (LAW-175-13). The server answers
 * `workspace_manage` for that workspace; until it answers, the answer is no (fail closed).
 */
export function useCanEditSelectedWorkspace(): boolean {
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const { data } = useSettingsAccess(tenantId, workspaceId);
  return data ? toSettingsAccess(data).workspaceManage : false;
}
