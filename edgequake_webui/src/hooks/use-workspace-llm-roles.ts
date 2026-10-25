"use client";

/**
 * SPEC-175 W4 — the workspace role draft (`llm_roles`) and its save.
 * The baseline is the workspace in the tenant store; a save writes the server's copy back there.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { updateWorkspace } from "@/lib/api/edgequake/workspaces";
import type { Workspace } from "@/types/workspace";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { UsageKey } from "@/lib/api/edgequake/model-catalog";
import {
  buildLlmRoles,
  draftFromWorkspaceRoles,
  isDraftDirty,
  type RoleDraft,
} from "@/lib/settings/model-usage";
import { modelPolicyQueryKeys } from "@/hooks/use-model-policy";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";

const EMPTY_ROLE: RoleDraft = { provider: "", model: "", connection_id: "" };

export function useWorkspaceLlmRoles(workspace: Workspace | null) {
  const saved = workspace?.llm_roles;
  const { draft, setDraft, reset } = useServerBackedDraft(saved, draftFromWorkspaceRoles);
  const savedDraft = draftFromWorkspaceRoles(saved);
  const queryClient = useQueryClient();

  const mutation = useMutation({
    mutationFn: () => {
      if (!workspace) throw new Error("Select a workspace first");
      return updateWorkspace(workspace.tenant_id, workspace.id, { llm_roles: buildLlmRoles(draft) });
    },
    onSuccess: (updated) => {
      if (!workspace) return;
      const latest = useTenantStore.getState().workspaces;
      useTenantStore.getState().setWorkspaces(
        latest.map((w) => (w.id === workspace.id ? { ...w, ...updated } : w)),
      );
      void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.effective(workspace.id) });
    },
  });

  const patchUsage = (usage: UsageKey, next: Partial<RoleDraft>) => {
    mutation.reset();
    setDraft((current) => ({ ...current, [usage]: { ...EMPTY_ROLE, ...current[usage], ...next } }));
  };

  return {
    draft,
    dirty: isDraftDirty(draft, savedDraft),
    patchUsage,
    reset,
    save: () => mutation.mutateAsync(),
    saving: mutation.isPending,
  };
}
