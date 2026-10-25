"use client";

/**
 * SPEC-175 You / Browser data. Backup, history and reset. Theme goes through next-themes
 * and language through `setLanguage`, so an import or reset keeps every surface in sync.
 */

import { SettingRow, SettingsFrame, SettingsGroup, SettingsNotice } from "@/components/settings/shell/settings-frame";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { useQueryStore } from "@/stores/use-query-store";
import { useSettingsStore } from "@/stores/use-settings-store";
import { Download, Upload } from "lucide-react";
import { useTheme } from "next-themes";
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

function downloadText(filename: string, text: string) {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

function ImportSettingsButton({ onFile }: { onFile: (file: File) => void }) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <>
      <Button variant="outline" size="sm" onClick={() => inputRef.current?.click()}>
        <Upload className="mr-2 h-4 w-4" aria-hidden="true" />
        {t("common.import", "Import")}
      </Button>
      <input
        ref={inputRef}
        type="file"
        accept=".json,application/json"
        className="sr-only"
        tabIndex={-1}
        aria-hidden="true"
        data-testid="settings-import-input"
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";
          if (file) onFile(file);
        }}
      />
    </>
  );
}

export function BrowserDataSection() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const exportSettings = useSettingsStore((s) => s.exportSettings);
  const importSettings = useSettingsStore((s) => s.importSettings);
  const resetSettings = useSettingsStore((s) => s.resetSettings);
  const setLanguage = useSettingsStore((s) => s.setLanguage);
  const clearHistory = useQueryStore((s) => s.clearHistory);

  const handleExport = () => {
    const current = theme === "light" || theme === "dark" ? theme : "system";
    downloadText(`edgequake-settings-${new Date().toISOString().split("T")[0]}.json`, exportSettings(current));
    toast.success(t("settings.shell.data.exported", "Settings exported"));
  };

  const handleImport = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = importSettings(String(reader.result ?? ""));
      if (!result.success) {
        toast.error(t("settings.shell.data.importError", "Import failed: {{error}}", { error: result.error }));
        return;
      }
      if (result.theme) setTheme(result.theme);
      toast.success(t("settings.shell.toasts.settingsImportedSuccess", "Settings imported"));
    };
    reader.onerror = () => toast.error(t("common.failed", "Could not read the file"));
    reader.readAsText(file);
  };

  const handleReset = () => {
    resetSettings();
    setTheme("system");
    setLanguage("en");
    toast.success(t("settings.shell.toasts.settingsReset", "Settings reset to defaults"));
  };

  const handleClearHistory = () => {
    clearHistory();
    toast.success(t("settings.shell.toasts.historyCleared", "Query history cleared"));
  };

  return (
    <SettingsFrame
      title={t("settings.shell.sections.browserData", "Browser data")}
      lead={t("settings.shell.data.subtitle", "Local to this browser. Your documents and graph are not affected.")}
    >
      <SettingsGroup title={t("settings.shell.data.backupGroup", "Backup")}>
        <SettingRow
          label={t("settings.shell.data.backup", "Settings backup")}
          description={t("settings.shell.data.backupDesc", "Export these preferences as JSON, or import a file you exported.")}
          control={() => (
            <>
              <Button variant="outline" size="sm" onClick={handleExport}>
                <Download className="mr-2 h-4 w-4" aria-hidden="true" />
                {t("common.export", "Export")}
              </Button>
              <ImportSettingsButton onFile={handleImport} />
            </>
          )}
        />
      </SettingsGroup>

      <SettingsGroup title={t("settings.shell.data.historyGroup", "History")}>
        <SettingRow
          label={t("settings.shell.data.queryHistory", "Query history")}
          description={t("settings.shell.data.queryHistoryDesc", "Saved queries and conversations in this browser.")}
          control={() => (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="outline" size="sm" data-testid="clear-history-trigger">
                  {t("settings.shell.data.clearHistory", "Clear history")}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{t("settings.shell.data.clearHistoryTitle", "Clear query history?")}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {t("settings.shell.data.clearHistoryDesc", "This permanently deletes your saved queries and favorites. It cannot be undone.")}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t("common.cancel", "Cancel")}</AlertDialogCancel>
                  <AlertDialogAction onClick={handleClearHistory} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                    {t("settings.shell.data.clearHistoryConfirm", "Clear")}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          )}
        />
      </SettingsGroup>

      <SettingsGroup title={t("settings.shell.data.resetGroup", "Reset")}>
        <SettingRow
          label={t("settings.shell.data.resetAll", "Reset preferences")}
          description={t("settings.shell.data.resetAllDesc", "Return theme, language and graph display to their defaults.")}
          control={() => (
            <AlertDialog>
              <AlertDialogTrigger asChild>
                <Button variant="destructive" size="sm" data-testid="reset-settings-trigger">
                  {t("settings.shell.data.resetButton", "Reset preferences")}
                </Button>
              </AlertDialogTrigger>
              <AlertDialogContent>
                <AlertDialogHeader>
                  <AlertDialogTitle>{t("settings.shell.data.resetConfirmTitle", "Reset your preferences?")}</AlertDialogTitle>
                  <AlertDialogDescription>
                    {t("settings.shell.data.resetConfirmDesc", "Theme, language and graph display go back to their defaults. Documents and the knowledge graph are not affected.")}
                  </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                  <AlertDialogCancel>{t("common.cancel", "Cancel")}</AlertDialogCancel>
                  <AlertDialogAction onClick={handleReset} className="bg-destructive text-destructive-foreground hover:bg-destructive/90">
                    {t("settings.shell.data.resetConfirm", "Reset")}
                  </AlertDialogAction>
                </AlertDialogFooter>
              </AlertDialogContent>
            </AlertDialog>
          )}
        />
      </SettingsGroup>

      <SettingsNotice testId="browser-data-note">
        {t("settings.shell.data.localNote", "These settings live in this browser. Use Export to move them to another one.")}
      </SettingsNotice>
    </SettingsFrame>
  );
}
