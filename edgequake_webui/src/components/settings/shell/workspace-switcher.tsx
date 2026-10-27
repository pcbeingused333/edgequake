"use client";

/**
 * SPEC-175 — the settings workspace switcher. It lists the workspaces the caller can
 * administer (the app header still lists every workspace the caller can open). Switching
 * keeps the current section, so Models stays Models (W-31, W-32).
 */

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { sectionHref, type SettingsSectionId } from "@/lib/settings/registry";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { Workspace } from "@/types";
import { useRouter } from "next/navigation";
import { useTranslation } from "react-i18next";

export function WorkspaceSwitcher({
  workspace,
  workspaces,
  activeSection,
  workspaceSlug,
}: {
  workspace: Workspace | null;
  workspaces: readonly Workspace[];
  activeSection: SettingsSectionId;
  workspaceSlug?: string | null;
}) {
  const { t } = useTranslation();
  const router = useRouter();
  const selectWorkspace = useTenantStore((s) => s.selectWorkspace);

  // No administered workspace, no switcher. The Models page is then read-only.
  if (workspaces.length === 0) return null;

  const onChange = (id: string) => {
    const next = workspaces.find((w) => w.id === id);
    if (!next) return;
    selectWorkspace(next.id);
    router.push(sectionHref(activeSection, workspaceSlug ? next.slug : null));
  };

  return (
    <div className="mt-3 px-2">
      <label className="mb-1 block text-[11px] font-medium text-muted-foreground" id="settings-switcher-label">
        {t("settings.switcher.label", "Switch workspace")}
      </label>
      <Select value={workspace?.id ?? undefined} onValueChange={onChange}>
        <SelectTrigger
          size="sm"
          aria-labelledby="settings-switcher-label"
          title={workspace?.name}
          className="w-full min-w-0 text-left *:data-[slot=select-value]:!block *:data-[slot=select-value]:!truncate *:data-[slot=select-value]:!text-left"
          data-testid="settings-workspace-switcher"
        >
          <SelectValue placeholder={t("settings.switcher.placeholder", "Choose a workspace")} />
        </SelectTrigger>
        <SelectContent>
          {workspaces.map((w) => (
            <SelectItem key={w.id} value={w.id} title={w.name}>
              <span className="truncate">{w.name}</span>
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
