"use client";

/**
 * SPEC-175 Organization. Members and roles live on the members page.
 * Provider grants live on Organization → Providers.
 */

import { MembersLinkCard } from "@/components/settings/members-link-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useTranslation } from "react-i18next";

export function PeopleSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.people", "People")}
      lead={t("settings.shell.people.lead", "Invite people and set their roles.")}
    >
      <MembersLinkCard />
    </SettingsFrame>
  );
}
