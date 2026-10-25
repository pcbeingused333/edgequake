"use client";

/**
 * SPEC-175 You / Appearance. Theme is written only through next-themes; language only
 * through the settings store's `setLanguage` (LAW-175-7, single writer per fact).
 */

import { SettingRow, SettingsFrame, SettingsGroup } from "@/components/settings/shell/settings-frame";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { useSettingsStore, type Language } from "@/stores/use-settings-store";
import { Monitor, Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useTranslation } from "react-i18next";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";

type ThemeValue = "light" | "dark" | "system";

const THEME_OPTIONS: readonly { value: ThemeValue; label: string; fallback: string; Icon: typeof Sun }[] = [
  { value: "light", label: "settings.appearance.themeLight", fallback: "Light", Icon: Sun },
  { value: "dark", label: "settings.appearance.themeDark", fallback: "Dark", Icon: Moon },
  { value: "system", label: "settings.appearance.themeSystem", fallback: "System", Icon: Monitor },
];

const LANGUAGE_OPTIONS: readonly { value: Language; name: string }[] = [
  { value: "en", name: "English" },
  { value: "fr", name: "Français" },
  { value: "zh", name: "中文" },
];

function ThemeOption({
  value,
  label,
  Icon,
  checked,
}: {
  value: ThemeValue;
  label: string;
  Icon: typeof Sun;
  checked: boolean;
}) {
  const id = `theme-${value}`;
  return (
    <label
      htmlFor={id}
      data-testid={`theme-option-${value}`}
      className={cn(
        "flex cursor-pointer flex-col gap-2 rounded-md border p-2 text-sm transition-colors",
        "hover:border-foreground/30 has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring",
        checked ? "border-primary bg-accent/50" : "border-border",
      )}
    >
      <span className="flex h-10 items-center justify-center rounded bg-muted text-muted-foreground" aria-hidden="true">
        <Icon className="h-4 w-4" />
      </span>
      <span className="flex items-center gap-2">
        <RadioGroupItem id={id} value={value} aria-label={label} />
        <span className="font-medium">{label}</span>
      </span>
    </label>
  );
}

export function AppearanceSection() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const language = useSettingsStore((s) => s.language);
  const setLanguage = useSettingsStore((s) => s.setLanguage);
  const current: ThemeValue = theme === "light" || theme === "dark" ? theme : "system";

  return (
    <SettingsFrame
      title={t("settings.shell.sections.appearance", "Appearance")}
      lead={t("settings.shell.appearance.subtitle", "Only you see these choices. They apply in this browser.")}
    >
      <SettingsGroup>
        <SettingRow
          label={t("settings.shell.appearance.theme", "Theme")}
          description={t("settings.shell.appearance.themeDesc", "Light, dark, or follow your system.")}
          testId="setting-theme"
          control={(labelId) => (
            <RadioGroup
              aria-labelledby={labelId}
              value={current}
              onValueChange={(next) => setTheme(next)}
              className="grid w-full grid-cols-3 gap-2 sm:w-[360px]"
            >
              {THEME_OPTIONS.map((option) => (
                <ThemeOption
                  key={option.value}
                  value={option.value}
                  label={t(option.label, option.fallback)}
                  Icon={option.Icon}
                  checked={current === option.value}
                />
              ))}
            </RadioGroup>
          )}
        />
        <SettingRow
          label={t("settings.shell.appearance.language", "Language")}
          description={t("settings.shell.appearance.languageDesc", "Interface language for this browser.")}
          testId="setting-language"
          control={(labelId) => (
            <Select value={language} onValueChange={(next) => setLanguage(next as Language)}>
              <SelectTrigger className="w-[180px]" aria-labelledby={labelId} data-testid="language-select">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LANGUAGE_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        />
      </SettingsGroup>
    </SettingsFrame>
  );
}
