"use client";

/** SPEC-175 Workspace / Access — document labels and clearance. Editors only. */

import { DocumentLabelsCard } from "@/components/settings/document-labels-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function AccessSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.access", "Access")}
      lead={t("settings.shell.access.lead", "Who can see which documents in this workspace, by label.")}
    >
      <DocumentLabelsCard />
    </SettingsFrame>
  );
}
