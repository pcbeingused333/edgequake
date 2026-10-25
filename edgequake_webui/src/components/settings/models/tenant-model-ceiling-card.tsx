"use client";

/**
 * SPEC-175 W5 — the organization's model ceiling. Tenant managers tick which registered models
 * each usage may use. A usage with no ticks is unconstrained. Workspaces can narrow the ceiling
 * but never widen it.
 */

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import {
  useSaveTenantModelPolicy,
  useTenantModelCatalog,
  useTenantModelPolicy,
} from "@/hooks/use-model-policy";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import { USAGE_KEYS, type UsageKey, type PolicyRow } from "@/lib/api/edgequake/model-catalog";
import {
  ceilingDisplayRows,
  ceilingRowsFromSelection,
  samePolicyRows,
  selectionFromCeilingRows,
  togglePair,
  type CeilingDisplayRow,
  type PairSelection,
  USAGE_LABELS,
} from "@/lib/settings/model-usage";
import { ModelSaveBar, saveWithToast } from "./model-save-bar";

const NO_ROWS: readonly PolicyRow[] = [];
const NO_SELECTION: PairSelection = {};

/** Soft tints, so a failed probe reads as a warning rather than a block of alarm colour. */
const HEALTH_BADGE: Record<
  NonNullable<CeilingDisplayRow["health"]>,
  { label: string; variant: "secondary" | "outline"; className?: string }
> = {
  healthy: { label: "Healthy", variant: "secondary", className: "text-emerald-700 dark:text-emerald-400" },
  probe_failed: {
    label: "Check failed",
    variant: "outline",
    className: "border-destructive/30 bg-destructive/10 text-destructive",
  },
  unknown: { label: "Not checked", variant: "outline", className: "text-muted-foreground" },
};

interface TenantModelCeilingCardProps {
  tenantId: string;
}

/** How many models a usage allows, in words. */
function allowedSummary(count: number): string {
  return count === 0 ? "Any model" : `${count} allowed`;
}

export function TenantModelCeilingCard({ tenantId }: TenantModelCeilingCardProps) {
  const catalog = useTenantModelCatalog(tenantId);
  const policy = useTenantModelPolicy(tenantId);
  const save = useSaveTenantModelPolicy(tenantId);

  const savedRows = policy.data?.rows ?? NO_ROWS;
  const { draft, setDraft, reset } = useServerBackedDraft(savedRows, selectionFromCeilingRows);
  const dirty = !samePolicyRows(ceilingRowsFromSelection(draft), savedRows);
  const displayRows = ceilingDisplayRows(catalog.data?.entries ?? [], draft);

  const countFor = (usage: UsageKey) => (draft[usage] ?? []).length;

  return (
    <Card data-testid="tenant-model-ceiling-card">
      <CardHeader>
        <CardTitle>Model ceiling</CardTitle>
        <CardDescription>
          The most this organization allows for each job. Workspaces can narrow it but never widen
          it. A job with no ticks can use any registered model.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {catalog.isPending ? (
          <p className="text-sm text-muted-foreground">Loading registered models…</p>
        ) : displayRows.length === 0 ? (
          <p className="text-sm text-muted-foreground">No models are registered on this deployment.</p>
        ) : (
          <div className="overflow-x-auto">
            <Table data-testid="tenant-model-ceiling-table">
              <TableHeader>
                <TableRow>
                  <TableHead className="min-w-56">Model</TableHead>
                  {USAGE_KEYS.map((usage) => (
                    <TableHead key={usage} className="text-center">
                      <div>{USAGE_LABELS[usage]}</div>
                      <div className="text-xs font-normal text-muted-foreground" data-testid={`ceiling-summary-${usage}`}>
                        {allowedSummary(countFor(usage))}
                      </div>
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {displayRows.map((row) => (
                  <TableRow key={row.key} data-testid={`ceiling-row-${row.key}`}>
                    <TableCell className="space-y-1">
                      <div className="font-medium">{row.label}</div>
                      {row.health ? (
                        <Badge variant={HEALTH_BADGE[row.health].variant} className={HEALTH_BADGE[row.health].className}>
                          {HEALTH_BADGE[row.health].label}
                        </Badge>
                      ) : (
                        <Badge variant="outline">Not registered</Badge>
                      )}
                    </TableCell>
                    {USAGE_KEYS.map((usage) => {
                      if (!row.usages.includes(usage)) {
                        return (
                          <TableCell key={usage} className="text-center text-muted-foreground">
                            —
                          </TableCell>
                        );
                      }
                      const checked = (draft[usage] ?? []).includes(row.key);
                      return (
                        <TableCell key={usage} className="text-center">
                          <Checkbox
                            aria-label={`Allow ${row.label} for ${USAGE_LABELS[usage]}`}
                            checked={checked}
                            onCheckedChange={() => setDraft((current) => togglePair(current, usage, row.key))}
                          />
                        </TableCell>
                      );
                    })}
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
        <div className="flex items-center justify-between gap-2">
          <Button
            type="button"
            variant="outline"
            disabled={save.isPending || ceilingRowsFromSelection(draft).length === 0}
            onClick={() => setDraft(() => NO_SELECTION)}
          >
            Clear all limits
          </Button>
          <ModelSaveBar
            dirty={dirty}
            saving={save.isPending}
            onDiscard={reset}
            onSave={() =>
              void saveWithToast(() => save.mutateAsync(ceilingRowsFromSelection(draft)), "Model ceiling saved")
            }
          />
        </div>
      </CardContent>
    </Card>
  );
}
