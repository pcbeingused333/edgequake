"use client";

/**
 * SPEC-175 Deployment / Fleet — server LLM defaults and live provider health.
 * One health view (ProviderStatusHub). The duplicate status card is retired.
 */

import { ProviderStatusHub } from "@/components/settings/provider-status-hub";
import { ServerLlmConfigCard } from "@/components/settings/server-llm-config-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { fetchProvidersHealth } from "@/lib/api/models";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";

export function FleetSection() {
  const { t } = useTranslation();
  const health = useQuery({
    queryKey: ["provider-health"],
    queryFn: fetchProvidersHealth,
    staleTime: 15_000,
  });
  return (
    <SettingsFrame
      title={t("settings.shell.sections.fleet", "Fleet")}
      lead={t("settings.shell.fleet.lead", "Server-wide model defaults and whether each provider is reachable.")}
    >
      <ServerLlmConfigCard />
      <ProviderStatusHub
        providers={health.data}
        isLoading={health.isLoading}
        onRefresh={() => void health.refetch()}
      />
    </SettingsFrame>
  );
}
