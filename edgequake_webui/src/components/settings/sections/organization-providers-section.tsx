"use client";

/** SPEC-177 Organization / Providers. Tenant managers only. */

import { ProviderOfferEditor } from "@/components/settings/providers/provider-offer-editor";
import { TenantModelCeilingCard } from "@/components/settings/models/tenant-model-ceiling-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { Button } from "@/components/ui/button";
import { useSettingsContext } from "@/hooks/use-settings-context";
import { modelPolicyQueryKeys } from "@/hooks/use-model-policy";
import { providerOfferKeys } from "@/hooks/use-provider-offer";
import { refreshDynamicModels } from "@/hooks/use-providers";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

function RefreshOrgModels({ tenantId }: { tenantId: string | null }) {
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const onClick = async () => {
    setBusy(true);
    try {
      await refreshDynamicModels(queryClient);
      if (tenantId) {
        void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.catalog(tenantId) });
        void queryClient.invalidateQueries({ queryKey: modelPolicyQueryKeys.tenantPolicy(tenantId) });
        void queryClient.invalidateQueries({ queryKey: providerOfferKeys.tenant(tenantId) });
      }
      toast.success("Models refreshed");
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Could not refresh models");
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="flex justify-end">
      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={busy}
        data-testid="org-refresh-models"
        onClick={() => void onClick()}
      >
        {busy ? "Refreshing…" : "Refresh models"}
      </Button>
    </div>
  );
}

export function OrganizationProvidersSection() {
  const { t } = useTranslation();
  const { tenantId } = useSettingsContext(null);
  return (
    <SettingsFrame
      title={t("settings.shell.sections.orgProviders", "Providers")}
      lead={t(
        "settings.shell.orgProviders.lead",
        "Choose the providers this organization may use, then the models for each job.",
      )}
    >
      <div className="space-y-6">
        <RefreshOrgModels tenantId={tenantId} />
        <ProviderOfferEditor parent="tenant" tenantId={tenantId} />
        {tenantId ? <TenantModelCeilingCard tenantId={tenantId} /> : null}
      </div>
    </SettingsFrame>
  );
}
