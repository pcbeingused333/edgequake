"use client";

/** SPEC-175 Models / Query defaults. Browser-local starting point for the query composer. */

import { SettingRow, SettingsGroup } from "@/components/settings/shell/settings-frame";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { QUERY_MODE_META } from "@/lib/query/query-mode-meta";
import { useSettingsStore } from "@/stores/use-settings-store";
import type { QueryMode } from "@/types";
import { useTranslation } from "react-i18next";

export function QueryDefaultsGroup() {
  const { t } = useTranslation();
  const query = useSettingsStore((s) => s.querySettings);
  const setQuery = useSettingsStore((s) => s.setQuerySettings);

  return (
    <SettingsGroup
      id="query-defaults"
      title={t("settings.shell.query.title", "Query defaults")}
      description={t("settings.shell.query.subtitle", "Starting values for new queries in this browser. The query composer can override them.")}
    >
      <SettingRow
        label={t("settings.shell.query.defaultMode", "Default query mode")}
        description={t("settings.shell.query.defaultModeDesc", "Retrieval mode used when a query opens.")}
        testId="setting-query-mode"
        control={(labelId) => (
          <Select value={query.mode} onValueChange={(mode) => setQuery({ mode: mode as QueryMode })}>
            <SelectTrigger className="w-[220px]" aria-labelledby={labelId}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {QUERY_MODE_META.map((mode) => (
                <SelectItem key={mode.id} value={mode.id} title={mode.description}>
                  {t(`query.modes.${mode.id}`, mode.label)}
                  <span className="ml-1 text-xs text-muted-foreground">({mode.apiName})</span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
      />
      <SettingRow
        label={t("settings.shell.query.enableStreaming", "Stream answers")}
        description={t("settings.shell.query.enableStreamingDesc", "Show the answer as it is generated.")}
        control={(labelId) => (
          <Switch
            aria-labelledby={labelId}
            checked={query.stream}
            onCheckedChange={(stream) => setQuery({ stream })}
          />
        )}
      />
    </SettingsGroup>
  );
}
