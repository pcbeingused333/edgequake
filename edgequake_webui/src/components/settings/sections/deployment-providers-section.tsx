"use client";

/** SPEC-177 Deployment / Providers. Platform admins only. */

import { ProviderOfferEditor } from "@/components/settings/providers/provider-offer-editor";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function DeploymentProvidersSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.providers", "Providers")}
      lead={t("settings.shell.providers.lead", "These are the providers organizations can turn on.")}
    >
      <ProviderOfferEditor parent="deployment" />
    </SettingsFrame>
  );
}
