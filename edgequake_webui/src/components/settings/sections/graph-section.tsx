"use client";

/** SPEC-175 You / Graph display. Browser-local; no model fields live here. */

import { SettingRow, SettingsFrame, SettingsGroup } from "@/components/settings/shell/settings-frame";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useSettingsStore } from "@/stores/use-settings-store";
import type { GraphSettings } from "@/types";
import { useTranslation } from "react-i18next";

type NodeSize = GraphSettings["nodeSize"];
type Layout = GraphSettings["layout"];

const NODE_SIZES: readonly { value: NodeSize; label: string }[] = [
  { value: "small", label: "Small" },
  { value: "medium", label: "Medium" },
  { value: "large", label: "Large" },
];

/** Both selects share one width so the right-hand column lines up. */
const SELECT_WIDTH = "w-[220px]";

const LAYOUTS: readonly { value: Layout; label: string }[] = [
  { value: "force", label: "ForceAtlas2 (FA2)" },
  { value: "force-directed", label: "Force-directed" },
  { value: "circular", label: "Circular" },
  { value: "circlepack", label: "Circle pack" },
  { value: "random", label: "Random" },
  { value: "noverlaps", label: "Noverlaps" },
  { value: "hierarchical", label: "Hierarchical" },
];

export function GraphSection() {
  const { t } = useTranslation();
  const graph = useSettingsStore((s) => s.graphSettings);
  const setGraph = useSettingsStore((s) => s.setGraphSettings);

  return (
    <SettingsFrame
      title={t("settings.shell.sections.graph", "Graph display")}
      lead={t("settings.shell.graph.subtitle", "How the knowledge graph looks in this browser.")}
    >
      <SettingsGroup>
        <SettingRow
          label={t("settings.shell.graph.showNodeLabels", "Node labels")}
          description={t("settings.shell.graph.showNodeLabelsDesc", "Show the name on each node.")}
          control={(labelId) => (
            <Switch
              aria-labelledby={labelId}
              checked={graph.showLabels}
              onCheckedChange={(showLabels) => setGraph({ showLabels })}
            />
          )}
        />
        <SettingRow
          label={t("settings.shell.graph.showEdgeLabels", "Edge labels")}
          description={t("settings.shell.graph.showEdgeLabelsDesc", "Show the relationship type on each edge.")}
          control={(labelId) => (
            <Switch
              aria-labelledby={labelId}
              checked={graph.showEdgeLabels}
              onCheckedChange={(showEdgeLabels) => setGraph({ showEdgeLabels })}
            />
          )}
        />
        <SettingRow
          label={t("settings.shell.graph.nodeSize", "Node size")}
          description={t("settings.shell.graph.nodeSizeDesc", "Size of nodes in the graph.")}
          control={(labelId) => (
            <Select value={graph.nodeSize} onValueChange={(nodeSize) => setGraph({ nodeSize: nodeSize as NodeSize })}>
              <SelectTrigger className={SELECT_WIDTH} aria-labelledby={labelId}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {NODE_SIZES.map((size) => (
                  <SelectItem key={size.value} value={size.value}>
                    {t(`settings.graph.nodeSize${size.label}`, size.label)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        />
        <SettingRow
          label={t("settings.shell.graph.defaultLayout", "Default layout")}
          description={t("settings.shell.graph.defaultLayoutDesc", "Layout used when the graph opens.")}
          control={(labelId) => (
            <Select value={graph.layout} onValueChange={(layout) => setGraph({ layout: layout as Layout })}>
              <SelectTrigger className="w-[220px]" aria-labelledby={labelId}>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {LAYOUTS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
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
