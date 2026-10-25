"use client";

/** SPEC-175 Deployment / Observability and quotas. Platform admins only. */

import { AdminQuotaSection } from "@/components/settings/admin-quota-section";
import { AppAttributionSettingsCard } from "@/components/settings/app-attribution-settings-card";
import { LangfuseObservabilityCard } from "@/components/settings/langfuse-observability-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function ObservabilitySection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.observability", "Observability")}
      lead={t("settings.shell.observability.lead", "Tracing, attribution and quotas for this deployment.")}
    >
      <LangfuseObservabilityCard />
      <AppAttributionSettingsCard />
      <AdminQuotaSection />
    </SettingsFrame>
  );
}
