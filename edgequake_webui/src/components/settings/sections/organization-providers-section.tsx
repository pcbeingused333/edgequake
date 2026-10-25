"use client";

/** SPEC-177 Organization / Providers. Tenant managers only. */

import { ProviderOfferEditor } from "@/components/settings/providers/provider-offer-editor";
import { TenantModelCeilingCard } from "@/components/settings/models/tenant-model-ceiling-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useSettingsContext } from "@/hooks/use-settings-context";
import { useTranslation } from "react-i18next";

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
        <ProviderOfferEditor parent="tenant" tenantId={tenantId} />
        {tenantId ? <TenantModelCeilingCard tenantId={tenantId} /> : null}
      </div>
    </SettingsFrame>
  );
}
