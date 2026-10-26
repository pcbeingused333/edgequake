"use client";

import {
  fetchDeploymentProviders,
  fetchTenantOffer,
  fetchWorkspaceOffer,
  putConnectionModels,
  putTenantOffer,
  putWorkspaceOffer,
  withdrawProvider,
  type ProviderOffer,
  type PutOfferBody,
} from "@/lib/api/edgequake/provider-offer";
import { modelPolicyQueryKeys } from "@/hooks/use-model-policy";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

export const providerOfferKeys = {
  deployment: ["deployment-providers"] as const,
  tenant: (tenantId: string) => ["tenant-provider-offer", tenantId] as const,
  workspace: (workspaceId: string) => ["workspace-provider-offer", workspaceId] as const,
};

function invalidateAfterOffer(queryClient: ReturnType<typeof useQueryClient>, workspaceId?: string) {
  void queryClient.invalidateQueries({ queryKey: providerOfferKeys.deployment });
  void queryClient.invalidateQueries({ queryKey: ["tenant-provider-offer"] });
  void queryClient.invalidateQueries({ queryKey: ["workspace-provider-offer"] });
  void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.eligibleRoot });
  void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.effectiveRoot });
  void queryClient.invalidateQueries({ queryKey: ["llm-models"] });
  void queryClient.invalidateQueries({ queryKey: ["embedding-models"] });
  if (workspaceId) {
    void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.workspacePolicy(workspaceId) });
  }
}

export function useDeploymentProviders() {
  return useQuery({
    queryKey: providerOfferKeys.deployment,
    queryFn: fetchDeploymentProviders,
  });
}

export function useWithdrawProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ slug, withdrawn, confirm }: { slug: string; withdrawn: boolean; confirm?: boolean }) =>
      withdrawProvider(slug, withdrawn, confirm === true),
    onSuccess: () => invalidateAfterOffer(queryClient),
  });
}

export function useSaveConnectionModels(slug: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (
      models: {
        id: string;
        supports_chat: boolean;
        supports_vision: boolean;
        supports_embedding: boolean;
        embedding_dimension?: number;
      }[],
    ) => putConnectionModels(slug, models),
    onSuccess: () => invalidateAfterOffer(queryClient),
  });
}

export function useTenantOffer(tenantId: string | null) {
  return useQuery({
    queryKey: tenantId ? providerOfferKeys.tenant(tenantId) : ["tenant-provider-offer", "none"],
    queryFn: () => fetchTenantOffer(tenantId as string),
    enabled: Boolean(tenantId),
  });
}

export function useSaveTenantOffer(tenantId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: PutOfferBody) => {
      const current = queryClient.getQueryData<ProviderOffer>(providerOfferKeys.tenant(tenantId));
      return putTenantOffer(tenantId, body, current?.revision);
    },
    onSuccess: (data) => {
      queryClient.setQueryData(providerOfferKeys.tenant(tenantId), data);
      invalidateAfterOffer(queryClient);
    },
  });
}

export function useWorkspaceOffer(workspaceId: string | null) {
  return useQuery({
    queryKey: workspaceId ? providerOfferKeys.workspace(workspaceId) : ["workspace-provider-offer", "none"],
    queryFn: () => fetchWorkspaceOffer(workspaceId as string),
    enabled: Boolean(workspaceId),
  });
}

export function useSaveWorkspaceOffer(workspaceId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: PutOfferBody) => {
      const current = queryClient.getQueryData<ProviderOffer>(providerOfferKeys.workspace(workspaceId));
      return putWorkspaceOffer(workspaceId, body, current?.revision);
    },
    onSuccess: (data: ProviderOffer) => {
      queryClient.setQueryData(providerOfferKeys.workspace(workspaceId), data);
      invalidateAfterOffer(queryClient, workspaceId);
    },
  });
}
