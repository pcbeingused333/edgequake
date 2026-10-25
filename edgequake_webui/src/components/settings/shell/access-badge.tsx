"use client";

/** SPEC-175 LAW-175-17 — the one badge that says what the caller may do here. */

import { Badge } from "@/components/ui/badge";
import { ACCESS_CHIP_COPY, accessChip, type SettingsAccess } from "@/lib/settings/access";
import { useTranslation } from "react-i18next";

export function AccessBadge({ access }: { access: SettingsAccess }) {
  const { t } = useTranslation();
  const chip = accessChip(access);
  const copy = ACCESS_CHIP_COPY[chip];
  return (
    <Badge variant="outline" className="shrink-0 text-[11px] font-medium" data-testid="settings-access-badge" data-chip={chip}>
      {t(copy.label, copy.fallback)}
    </Badge>
  );
}
