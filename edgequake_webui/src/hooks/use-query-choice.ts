"use client";

import { useScopedEligibleModels, useEffectiveUsages } from "@/hooks/use-model-policy";
import type { EligibleModel, ResolvedUsage } from "@/lib/api/edgequake/model-catalog";
import {
  choiceSendBlocked,
  inheritIsPossible,
} from "@/lib/query/choice-possibility";
import { useSettingsStore, visibleChoiceModels } from "@/stores/use-settings-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import { useEffect, useMemo } from "react";

/** Workspace query choice: eligible models, the effective row, lock, and an empty set. */
export function useQueryChoice() {
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const eligible = useScopedEligibleModels({
    scope: "workspace",
    id: workspaceId,
    usage: "query",
  });
  const effective = useEffectiveUsages(workspaceId);
  const adoptLegacyQueryModel = useSettingsStore((s) => s.adoptLegacyQueryModel);
  const queryUsage = (effective.data?.usages ?? []).find((row) => row.usage === "query");
  const models = visibleChoiceModels(eligible.data?.models ?? []);
  const override = useSettingsStore((s) =>
    workspaceId ? s.queryModelsByWorkspace[workspaceId] : undefined,
  );
  const inheritOk = useMemo(
    () => inheritIsPossible(queryUsage, models),
    [models, queryUsage],
  );
  const empty = eligible.isSuccess && models.length === 0;
  const sendBlocked = choiceSendBlocked({
    eligibleLoaded: eligible.isSuccess,
    eligibleEmpty: empty,
    inheritOk,
    overrideProvider: override?.provider,
    overrideModel: override?.model,
  });

  useEffect(() => {
    if (!workspaceId) return;
    adoptLegacyQueryModel(workspaceId);
    const state = useSettingsStore.getState();
    const slot = state.queryModelsByWorkspace[workspaceId];
    state.setQuerySettings({ provider: slot?.provider, model: slot?.model });
  }, [adoptLegacyQueryModel, workspaceId]);

  return {
    workspaceId,
    eligible,
    queryUsage,
    models,
    inheritOk,
    locked: queryUsage?.locked === true,
    empty,
    sendBlocked,
  };
}

export type QueryChoiceModel = EligibleModel;
export type QueryChoiceUsage = ResolvedUsage;
