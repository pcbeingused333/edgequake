"use client";

/**
 * SPEC-175 — below `md` the nav becomes two selects: scope, then section. Same registry,
 * same gates as the desktop nav (DRY). Changing either select navigates.
 */

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import type { SettingsAccess } from "@/lib/settings/access";
import {
  getSection,
  scopeHref,
  sectionHref,
  visibleScopes,
  visibleSections,
  type SettingsScope,
  type SettingsSectionId,
} from "@/lib/settings/registry";
import { useRouter } from "next/navigation";

export function MobileSettingsNav({
  access,
  activeSection,
  workspaceSlug,
}: {
  access: SettingsAccess;
  activeSection: SettingsSectionId;
  workspaceSlug: string | null;
}) {
  const router = useRouter();
  const active = getSection(activeSection);
  const scopes = visibleScopes(access);
  const sections = visibleSections(access, active.scope);

  return (
    <div className="grid grid-cols-2 gap-2" data-testid="settings-mobile-nav">
      <Select
        value={active.scope}
        onValueChange={(value) => router.push(scopeHref(value as SettingsScope, workspaceSlug))}
      >
        <SelectTrigger aria-label="Settings scope" className="h-8 text-sm" data-testid="settings-mobile-scope">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {scopes.map((scope) => (
            <SelectItem key={scope.id} value={scope.id}>
              {scope.fallback}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <Select
        value={activeSection}
        onValueChange={(value) => router.push(sectionHref(value as SettingsSectionId, workspaceSlug))}
      >
        <SelectTrigger aria-label="Settings section" className="h-8 text-sm" data-testid="settings-mobile-section">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {sections.map((section) => (
            <SelectItem key={section.id} value={section.id}>
              {section.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
