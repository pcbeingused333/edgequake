"use client";

/**
 * SPEC-175 Organization. Members, roles and SSO live on the members page. Tenant managers also
 * set the organization's model ceiling here.
 */

import { TenantModelCeilingCard } from "@/components/settings/models/tenant-model-ceiling-card";
import { MembersLinkCard } from "@/components/settings/members-link-card";
import { SettingsFrame } from "@/components/settings/shell/settings-frame";
import { useSettingsContext } from "@/hooks/use-settings-context";
import { useTranslation } from "react-i18next";

export function PeopleSection() {
  const { t } = useTranslation();
  const { access, tenantId } = useSettingsContext(null);
  return (
    <SettingsFrame
      title={t("settings.shell.sections.people", "People")}
      lead={t("settings.shell.people.lead", "Invite people, set their roles, and cap the models the organization can use.")}
    >
      <MembersLinkCard />
      {access.tenantManage && tenantId ? <TenantModelCeilingCard tenantId={tenantId} /> : null}
    </SettingsFrame>
  );
}
