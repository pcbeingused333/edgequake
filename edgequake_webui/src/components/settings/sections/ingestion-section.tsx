"use client";

/** SPEC-175 Workspace / Ingestion — editors only (gate: workspaceEditor). */

import { PdfParserSettingsCard } from "@/components/settings/pdf-parser-settings-card";
import { VisionLLMSettingsCard } from "@/components/settings/vision-llm-settings-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function IngestionSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.ingestion", "Ingestion")}
      lead={t("settings.shell.ingestion.lead", "How documents are read and converted before extraction. Applies to new uploads in this workspace.")}
    >
      <PdfParserSettingsCard />
      <VisionLLMSettingsCard />
    </SettingsFrame>
  );
}
