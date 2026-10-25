"use client";

import { useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import type { ConnectionOption } from "@/hooks/use-model-policy";
import { selectableConnections } from "@/lib/settings/connection-options";
import { useEligibleModels } from "@/hooks/use-model-policy";
import type { ResolvedUsage, UsageKey } from "@/lib/api/edgequake/model-catalog";
import {
  applyPick,
  brokenMessage,
  pairKey,
  pickerGroups,
  pickerValue,
  type RoleDraft,
  SOURCE_LABELS,
  USAGE_LABELS,
} from "@/lib/settings/model-usage";

interface UsageRoleRowProps {
  usage: UsageKey;
  workspaceId: string;
  /** Undefined for a usage with no role to write (`decision`): the row only reports. */
  draft?: RoleDraft;
  resolved: ResolvedUsage | undefined;
  connections: readonly ConnectionOption[];
  readOnly: boolean;
  onChange: (next: RoleDraft) => void;
}

const selectClass =
  "w-full rounded-md border bg-background px-2 py-1.5 text-sm disabled:cursor-not-allowed disabled:opacity-70";

/** The resolved pair for a usage, in words: "Ollama · gemma3 — Workspace role". */
function ResolvedSummary({ resolved }: { resolved: ResolvedUsage | undefined }) {
  if (!resolved) return <p className="text-xs text-muted-foreground">Not resolved yet.</p>;
  const broken = brokenMessage(resolved.broken_reason);
  return (
    <div className="space-y-1 text-xs">
      <p className="text-muted-foreground" data-testid="usage-resolved">
        Using {resolved.provider_display_name ?? resolved.provider} · {resolved.model} —{" "}
        {SOURCE_LABELS[resolved.source]}
      </p>
      {resolved.locked ? <Badge variant="outline">Set by the operator</Badge> : null}
      {broken ? (
        <p role="alert" className="text-destructive" data-testid="usage-broken">
          {broken}
        </p>
      ) : null}
    </div>
  );
}

/** One usage: resolved pair, eligible-only model picker, and optional connection. */
export function UsageRoleRow({
  usage,
  workspaceId,
  draft,
  resolved,
  connections,
  readOnly,
  onChange,
}: UsageRoleRowProps) {
  const label = USAGE_LABELS[usage];
  const eligible = useEligibleModels(draft ? workspaceId : null, usage);
  const groups = useMemo(() => pickerGroups(eligible.data?.models ?? []), [eligible.data]);
  const options = useMemo(() => groups.flatMap((g) => g.options), [groups]);

  if (!draft) {
    return (
      <div data-testid={`usage-row-${usage}`} className="py-3">
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted-foreground">Follows the workspace default. It has no role of its own.</p>
        <ResolvedSummary resolved={resolved} />
      </div>
    );
  }

  const value = pickerValue(draft);
  // Only judge eligibility once the list has loaded; a pending list would flash a false warning.
  const savedButIneligible = !eligible.isPending && !!value && !options.some((o) => o.value === value);
  const locked = resolved?.locked ?? false;
  const controlsDisabled = readOnly || locked;

  return (
    <div
      data-testid={`usage-row-${usage}`}
      className="grid gap-3 py-3 md:grid-cols-[minmax(0,1fr)_minmax(0,1.3fr)_minmax(0,1fr)] md:items-start"
    >
      <div className="space-y-1">
        <p className="text-sm font-medium">{label}</p>
        <ResolvedSummary resolved={resolved} />
      </div>
      <div className="space-y-1">
        <label className="text-xs text-muted-foreground" htmlFor={`usage-model-${usage}`}>
          Model
        </label>
        <select
          id={`usage-model-${usage}`}
          data-testid={`usage-model-${usage}`}
          aria-label={`Model for ${label}`}
          className={selectClass}
          value={value}
          disabled={controlsDisabled || eligible.isPending}
          onChange={(e) => onChange(applyPick(draft, options.find((o) => o.value === e.target.value)))}
        >
          <option value="">{eligible.isPending ? "Loading models…" : "Inherit the workspace default"}</option>
          {savedButIneligible ? (
            <option value={pairKey(draft.provider, draft.model)} disabled>
              {draft.model} (not eligible)
            </option>
          ) : null}
          {groups.map((group) => (
            <optgroup key={group.provider} label={group.label}>
              {group.options.map((o) => (
                <option key={o.value} value={o.value}>
                  {o.label}
                  {o.supportsVision ? " · vision" : ""}
                </option>
              ))}
            </optgroup>
          ))}
        </select>
        {savedButIneligible ? (
          <p className="text-xs text-destructive">
            The saved model is no longer allowed here. Pick another one to save.
          </p>
        ) : null}
      </div>
      <div className="space-y-1">
        <label className="text-xs text-muted-foreground" htmlFor={`usage-connection-${usage}`}>
          Connection
        </label>
        <select
          id={`usage-connection-${usage}`}
          data-testid={`usage-connection-${usage}`}
          aria-label={`Connection for ${label}`}
          className={selectClass}
          value={draft.connection_id}
          disabled={controlsDisabled}
          onChange={(e) => onChange({ ...draft, connection_id: e.target.value })}
        >
          <option value="">Inherit / environment</option>
          {selectableConnections(connections).map((c) => (
            <option key={`${c.source}-${c.slug}-${c.id}`} value={c.id}>
              {c.display_name}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}
