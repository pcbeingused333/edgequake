"use client";

/**
 * SPEC-175 W5 — workspace narrowing. Editors turn off models this workspace must not use, per
 * usage. The organization ceiling still applies: a model the ceiling forbids is not listed here.
 */

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useEligibleModelsByUsage, useSaveWorkspaceModelPolicy, useWorkspaceModelPolicy } from "@/hooks/use-model-policy";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import { USAGE_KEYS, type PolicyRow } from "@/lib/api/edgequake/model-catalog";
import {
  disabledFromRows,
  narrowingMatrix,
  narrowingRowsFromDisabled,
  samePolicyRows,
  togglePair,
  USAGE_LABELS,
} from "@/lib/settings/model-usage";
import { ModelSaveBar, saveWithToast } from "./model-save-bar";

const NO_ROWS: readonly PolicyRow[] = [];

interface WorkspaceModelNarrowingCardProps {
  workspaceId: string;
  /** Members see the grid; only editors can change it. */
  readOnly?: boolean;
}

export function WorkspaceModelNarrowingCard({ workspaceId, readOnly = false }: WorkspaceModelNarrowingCardProps) {
  const policy = useWorkspaceModelPolicy(workspaceId);
  const eligible = useEligibleModelsByUsage(workspaceId);
  const save = useSaveWorkspaceModelPolicy(workspaceId);

  const savedRows = policy.data?.rows ?? NO_ROWS;
  const { draft, setDraft, reset } = useServerBackedDraft(savedRows, disabledFromRows);
  const dirty = !samePolicyRows(narrowingRowsFromDisabled(draft), savedRows);
  const matrix = narrowingMatrix(eligible.byUsage, draft);
  const locked = readOnly || policy.data?.can_manage === false;

  return (
    <Card data-testid="workspace-model-narrowing-card">
      <CardHeader>
        <CardTitle>Narrow models for this workspace</CardTitle>
        <CardDescription>
          Turn off a model for a job this workspace should not use. Models the organization does not
          allow are not listed.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {eligible.loading || policy.isPending ? (
          <p className="text-sm text-muted-foreground">Loading models…</p>
        ) : matrix.length === 0 ? (
          <p className="text-sm text-muted-foreground">No models are available to this workspace yet.</p>
        ) : (
          <div
            className="overflow-x-auto"
            role="region"
            tabIndex={0}
            aria-label="Models by job. Scroll sideways for more jobs."
          >
            <Table data-testid="workspace-narrowing-table">
              <TableHeader>
                <TableRow>
                  <TableHead className="sticky left-0 z-10 min-w-40 bg-card">Model</TableHead>
                  {USAGE_KEYS.map((usage) => (
                    <TableHead key={usage} className="h-auto min-w-20 py-2 text-center text-xs leading-tight whitespace-normal">
                      {USAGE_LABELS[usage]}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <TableBody>
                {matrix.map((row) => (
                  <TableRow key={row.value} data-testid={`narrowing-row-${row.value}`}>
                    <TableCell className="sticky left-0 z-10 min-w-40 bg-card font-medium">
                      {row.provider} · {row.label}
                    </TableCell>
                    {USAGE_KEYS.map((usage) => {
                      if (!row.presentIn.has(usage)) {
                        return (
                          <TableCell key={usage} className="text-center text-muted-foreground">
                            —
                          </TableCell>
                        );
                      }
                      const allowed = !(draft[usage] ?? []).includes(row.value);
                      return (
                        <TableCell key={usage} className="text-center">
                          <Checkbox
                            aria-label={`Use ${row.label} for ${USAGE_LABELS[usage]}`}
                            checked={allowed}
                            disabled={locked}
                            onCheckedChange={() => setDraft((current) => togglePair(current, usage, row.value))}
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
        {locked ? (
          <p role="note" className="text-sm text-muted-foreground">
            Only workspace owners and admins can narrow these models.
          </p>
        ) : (
          <ModelSaveBar
            dirty={dirty}
            saving={save.isPending}
            onDiscard={reset}
            onSave={() =>
              void saveWithToast(() => save.mutateAsync(narrowingRowsFromDisabled(draft)), "Workspace models saved")
            }
          />
        )}
      </CardContent>
    </Card>
  );
}
