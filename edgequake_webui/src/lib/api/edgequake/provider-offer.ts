import { apiClient } from "@/lib/api/client";

export interface OfferModel {
  id: string;
  usages: string[];
}

export interface DeploymentProvider {
  slug: string;
  display_name: string;
  origin: string;
  api_shape: string;
  offered: boolean;
  withdrawn: boolean;
  probe: string;
  key_configured: boolean;
  models: OfferModel[];
}

export interface OfferProvider {
  slug: string;
  display_name: string;
  offered_by_parent: boolean;
  enabled: boolean;
  models: OfferModel[];
}

export interface ProviderOffer {
  tenant_id: string;
  workspace_id?: string | null;
  offer_mode: "unconstrained" | "constrained";
  parent_count: number;
  enabled_count: number;
  can_manage: boolean;
  providers: OfferProvider[];
  legacy: { slug: string; display_name: string }[];
}

export interface PutOfferBody {
  offer_mode: "unconstrained" | "constrained";
  slugs: string[];
}

export const fetchDeploymentProviders = () =>
  apiClient<{ providers: DeploymentProvider[] }>("/deployment/providers");

export const withdrawProvider = (slug: string, withdrawn: boolean) =>
  apiClient<DeploymentProvider>(`/deployment/providers/${encodeURIComponent(slug)}`, {
    method: "PUT",
    body: JSON.stringify({ withdrawn }),
  });

export const putConnectionModels = (
  slug: string,
  models: { id: string; supports_chat: boolean; supports_vision: boolean; supports_embedding: boolean }[],
) =>
  apiClient<void>(`/deployment/providers/${encodeURIComponent(slug)}/models`, {
    method: "PUT",
    body: JSON.stringify({ models }),
  });

export const fetchTenantOffer = (tenantId: string) =>
  apiClient<ProviderOffer>(`/tenants/${tenantId}/provider-offer`);

export const putTenantOffer = (tenantId: string, body: PutOfferBody) =>
  apiClient<ProviderOffer>(`/tenants/${tenantId}/provider-offer`, {
    method: "PUT",
    body: JSON.stringify(body),
  });

export const fetchWorkspaceOffer = (workspaceId: string) =>
  apiClient<ProviderOffer>(`/workspaces/${workspaceId}/provider-offer`);

export const putWorkspaceOffer = (workspaceId: string, body: PutOfferBody) =>
  apiClient<ProviderOffer>(`/workspaces/${workspaceId}/provider-offer`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
