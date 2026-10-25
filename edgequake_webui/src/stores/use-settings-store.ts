/**
 * @module use-settings-store
 * @description Zustand store for browser-local display and query preferences.
 *
 * SPEC-175 store split (LAW-175-7, LAW-175-16):
 * - Theme is owned by `next-themes` only. This store has no theme field.
 * - Language has one writer, `setLanguage` (zustand + i18next + <html lang>).
 * - Rerank and gleaning had no server reader and are removed. Version 2 drops them.
 * - Query provider and model default to undefined (inherit) and are cleared when
 *   the workspace changes (see query chip, EC-175-28).
 *
 * @implements FEAT0617 - User preference persistence
 * @implements FEAT0101-0104 - Query mode defaults
 * @implements FEAT0618 - Graph visualization settings
 *
 * @enforces BR0611 - Settings sync across tabs
 *
 * @see {@link specs/175-perfect-settings/10-frontend-architecture.md}
 */
"use client";

import { STORE_VERSIONS, ZUSTAND_STORAGE_KEYS } from "@/lib/storage-keys";
import type { GraphSettings, QueryMode, QuerySettings } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

const defaultGraphSettings: GraphSettings = {
  showLabels: true,
  showEdgeLabels: false,
  nodeSize: "medium",
  edgeThickness: "medium",
  layout: "force",
  colorBy: "type",
  enableNodeDrag: true,
  highlightNeighbors: true,
  hideUnselectedEdges: false,
};

const defaultQuerySettings: QuerySettings = {
  mode: "mix" as QueryMode,
  topK: 10,
  maxTokens: 2048,
  temperature: 0.7,
  stream: true,
  // Inherit: the server picks the model unless the composer chip sets a pair.
  provider: undefined,
  model: undefined,
  systemPrompt: undefined,
  fullChunkContent: false,
};

/** Keys that older builds persisted and no longer read. Dropped on migrate. */
const RETIRED_QUERY_KEYS = ["enableRerank", "rerankTopK"] as const;
/** Top-level persisted keys with no reader: theme (next-themes owns it) and gleaning. */
const RETIRED_TOP_LEVEL_KEYS = ["theme", "ingestionSettings"] as const;

export type Language = "en" | "zh" | "fr";

/** Shape written by `exportSettings` and read by `importSettings`. */
export interface SettingsExportFile {
  version: string;
  exportedAt: string;
  application: "EdgeQuake";
  settings: {
    theme?: "light" | "dark" | "system";
    language: Language;
    graphSettings: GraphSettings;
    querySettings: Partial<QuerySettings>;
    sidebarCollapsed: boolean;
  };
}

export interface SettingsState {
  language: Language;
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
  sidebarCollapsed: boolean;
  /** Tracks if store has been hydrated from localStorage */
  _hasHydrated: boolean;
}

interface SettingsActions {
  setLanguage: (language: Language) => void;
  setGraphSettings: (settings: Partial<GraphSettings>) => void;
  setQuerySettings: (settings: Partial<QuerySettings>) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleSidebar: () => void;
  resetSettings: () => void;
  setHasHydrated: (hydrated: boolean) => void;
  exportSettings: (theme?: SettingsExportFile["settings"]["theme"]) => string;
  importSettings: (jsonString: string) => ImportResult;
}

export interface ImportResult {
  success: boolean;
  error?: string;
  /** Theme from the file. The caller applies it through next-themes. */
  theme?: SettingsExportFile["settings"]["theme"];
}

type SettingsStore = SettingsState & SettingsActions;

const initialState: SettingsState = {
  language: "en",
  graphSettings: defaultGraphSettings,
  querySettings: defaultQuerySettings,
  sidebarCollapsed: false,
  _hasHydrated: false,
};

/** Removes keys that no reader uses anymore. Pure, so it is unit-tested. */
export function dropRetiredSettingsKeys(persisted: Record<string, unknown>): Record<string, unknown> {
  const rest = { ...persisted };
  for (const key of RETIRED_TOP_LEVEL_KEYS) delete rest[key];
  const query = { ...((rest.querySettings as Record<string, unknown>) ?? {}) };
  for (const key of RETIRED_QUERY_KEYS) delete query[key];
  return { ...rest, querySettings: query };
}

function applyImportedSettings(settings: SettingsExportFile["settings"]): Partial<SettingsState> {
  const query = dropRetiredSettingsKeys({ querySettings: settings.querySettings })
    .querySettings as Partial<QuerySettings>;
  return {
    language: settings.language || "en",
    graphSettings: { ...defaultGraphSettings, ...settings.graphSettings },
    querySettings: { ...defaultQuerySettings, ...query },
    sidebarCollapsed: settings.sidebarCollapsed ?? false,
  };
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      ...initialState,

      setLanguage: (language) => {
        set({ language });
        // SPEC-155 F-155-I01: keep i18next + <html lang> in sync
        if (typeof window !== "undefined") {
          void import("@/lib/i18n").then(({ default: i18n }) => {
            void i18n.changeLanguage(language);
            document.documentElement.lang = language;
          });
          try {
            localStorage.setItem("edgequake-language", language);
          } catch {
            /* ignore quota */
          }
        }
      },

      setGraphSettings: (settings) =>
        set((state) => ({
          graphSettings: { ...state.graphSettings, ...settings },
        })),

      setQuerySettings: (settings) =>
        set((state) => ({
          querySettings: { ...state.querySettings, ...settings },
        })),

      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      resetSettings: () => set(initialState),

      setHasHydrated: (hydrated) => set({ _hasHydrated: hydrated }),

      exportSettings: (theme) => {
        const state = get();
        const exportData: SettingsExportFile = {
          version: "2.0",
          exportedAt: new Date().toISOString(),
          application: "EdgeQuake",
          settings: {
            theme,
            language: state.language,
            graphSettings: state.graphSettings,
            querySettings: {},
            sidebarCollapsed: state.sidebarCollapsed,
          },
        };
        return JSON.stringify(exportData, null, 2);
      },

      importSettings: (jsonString: string) => {
        try {
          const data = JSON.parse(jsonString) as Partial<SettingsExportFile>;
          if (!data.version || !data.settings) {
            throw new Error("Invalid settings file format");
          }
          if (data.application && data.application !== "EdgeQuake") {
            throw new Error("Settings file is from a different application");
          }
          set(applyImportedSettings(data.settings as SettingsExportFile["settings"]));
          return { success: true, theme: data.settings.theme };
        } catch (error) {
          return {
            success: false,
            error: error instanceof Error ? error.message : "Unknown error",
          };
        }
      },
    }),
    {
      name: ZUSTAND_STORAGE_KEYS.SETTINGS_STORE,
      version: STORE_VERSIONS[ZUSTAND_STORAGE_KEYS.SETTINGS_STORE],
      partialize: (state) => ({
        language: state.language,
        graphSettings: state.graphSettings,
        querySettings: state.querySettings,
        sidebarCollapsed: state.sidebarCollapsed,
      }),
      /** v1 persisted theme, gleaning and rerank. v2 drops them (EC-175-23). */
      migrate: (persistedState: unknown) => {
        if (!persistedState || typeof persistedState !== "object") {
          return initialState;
        }
        return dropRetiredSettingsKeys(persistedState as Record<string, unknown>) as Partial<SettingsState>;
      },
      /** Deep-merge nested objects so a partial persisted blob keeps every default. */
      merge: (persistedState, currentState) => {
        if (!persistedState) {
          return currentState;
        }
        const persisted = persistedState as Partial<SettingsState>;
        return {
          ...currentState,
          ...persisted,
          graphSettings: {
            ...defaultGraphSettings,
            ...(persisted.graphSettings || {}),
          },
          querySettings: {
            ...defaultQuerySettings,
            ...(persisted.querySettings || {}),
          },
        };
      },
      onRehydrateStorage: () => {
        return (state, error) => {
          if (error) {
            console.error("[SettingsStore] Hydration failed:", error);
          }
          state?.setHasHydrated(true);
        };
      },
    },
  ),
);

/** Selector for hydration state */
export const useSettingsStoreHydrated = () => {
  return useSettingsStore((state) => state._hasHydrated);
};

export default useSettingsStore;
