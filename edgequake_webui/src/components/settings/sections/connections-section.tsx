"use client";

/** SPEC-175 Deployment / Connections. Platform admins only. */

import { ConnectionsCard } from "@/components/settings/connections-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function ConnectionsSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.connections", "Connections")}
      lead={t("settings.shell.connections.lead", "Saved provider connections. Keys are stored on the server and never shown again.")}
    >
      <ConnectionsCard />
    </SettingsFrame>
  );
}
