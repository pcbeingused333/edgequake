"use client";

/**
 * SPEC-175 Workspace / Models — every member can read; only a workspace owner or admin
 * (server `workspace_manage`) can change the models (LAW-175-13). Each job's model is picked
 * from the eligible list, and editors can narrow the list for this workspace.
 */

import { ProviderOfferEditor } from "@/components/settings/providers/provider-offer-editor";
import { QueryDefaultsGroup } from "@/components/settings/sections/query-defaults-group";
import { WorkspaceModelNarrowingCard } from "@/components/settings/models/workspace-model-narrowing-card";
import { WorkspaceRoleEditor } from "@/components/settings/models/workspace-role-editor";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { RebuildEmbeddingsButton } from "@/components/workspace/rebuild-embeddings-button";
import { useCanEditSelectedWorkspace } from "@/hooks/use-settings-context";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useTranslation } from "react-i18next";

export function WorkspaceModelsSection() {
  const { t } = useTranslation();
  const canEdit = useCanEditSelectedWorkspace();
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  return (
    <SettingsFrame
      title={t("settings.shell.sections.models", "Models")}
      lead={t("settings.shell.models.lead", "The models this workspace uses for each job. Members can read these; editors can change them.")}
    >
      <ProviderOfferEditor parent="workspace" workspaceId={workspaceId} readOnly={!canEdit} />
      <WorkspaceRoleEditor readOnly={!canEdit} />
      {workspaceId ? <WorkspaceModelNarrowingCard workspaceId={workspaceId} readOnly={!canEdit} /> : null}
      {canEdit ? <RebuildEmbeddingsButton variant="card" /> : null}
      <QueryDefaultsGroup />
    </SettingsFrame>
  );
}
