"use client";

/**
 * SPEC-179 — one Deployment provider: who it is, whether it works, when its
 * model list was last refreshed, and what organizations may see of it.
 */

import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Check, KeyRound, TriangleAlert } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { relativeTime } from "@/lib/members/format";
import type { DeploymentProvider } from "@/lib/api/edgequake/provider-offer";
import {
  capabilitiesOf,
  offerState,
  probeTone,
  refreshSummary,
  shortModelId,
} from "@/lib/settings/provider-status";

const MODELS_SHOWN = 6;

function StatusBadges({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const tone = probeTone(row.probe);
  const state = offerState(row);
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <Badge variant="outline" className="font-normal">
        {row.origin === "env" ? t("settings.providers.origin.env", "Environment") : t("settings.providers.origin.fleet", "Custom")}
      </Badge>
      <Badge
        variant="outline"
        data-testid={`probe-${row.slug}`}
        className={cn(
          "font-normal",
          tone === "ok" && "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
          tone === "failed" && "border-destructive/40 bg-destructive/10 text-destructive",
        )}
      >
        {tone === "ok"
          ? t("settings.providers.probe.ok", "Healthy")
          : tone === "failed"
            ? t("settings.providers.probe.failed", "Check failed")
            : t("settings.providers.probe.unchecked", "Not tested")}
      </Badge>
      <Badge variant="outline" className="font-normal" data-testid={`offer-state-${row.slug}`}>
        {state === "hidden"
          ? t("settings.providers.state.hidden", "Hidden from organizations")
          : state === "offered"
            ? t("settings.providers.state.offered", "Offered")
            : t("settings.providers.state.notOffered", "Not offered")}
      </Badge>
    </div>
  );
}

function RefreshLine({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const summary = refreshSummary(row);
  if (summary.tone === "never") {
    return (
      <p className="text-xs text-muted-foreground" data-testid={`refresh-line-${row.slug}`}>
        {t("settings.providers.neverRefreshed", "Model list not refreshed yet")}
      </p>
    );
  }
  const when = row.refresh ? relativeTime(row.refresh.refreshed_at) : "";
  return (
    <p className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground" data-testid={`refresh-line-${row.slug}`}>
      {summary.tone === "error" ? (
        <span className="inline-flex items-center gap-1 text-destructive" role="status">
          <TriangleAlert className="h-3.5 w-3.5" aria-hidden />
          {t("settings.providers.refreshFailedAt", "Last refresh failed {{when}}", { when })}
          {row.refresh?.error ? `: ${row.refresh.error}` : ""}
        </span>
      ) : (
        <span>{t("settings.providers.refreshedAt", "Refreshed {{when}}", { when })}</span>
      )}
      {summary.added > 0 ? (
        <Badge variant="secondary" className="px-1.5 py-0 text-[10px] font-normal" data-testid={`diff-added-${row.slug}`}>
          {t("settings.providers.diffAdded", "+{{count}} new", { count: summary.added })}
        </Badge>
      ) : null}
      {summary.removed > 0 ? (
        <Badge variant="secondary" className="px-1.5 py-0 text-[10px] font-normal" data-testid={`diff-removed-${row.slug}`}>
          {t("settings.providers.diffRemoved", "−{{count}} gone", { count: summary.removed })}
        </Badge>
      ) : null}
    </p>
  );
}

function ModelChips({ row }: { row: DeploymentProvider }) {
  const { t } = useTranslation();
  const [all, setAll] = useState(false);
  const shown = all ? row.models : row.models.slice(0, MODELS_SHOWN);
  if (row.models.length === 0) {
    return (
      <p className="text-xs text-muted-foreground">
        {t("settings.providers.noModelsYet", "No registered models. Refresh to discover them.")}
      </p>
    );
  }
  return (
    <ul className="flex flex-wrap gap-1.5" aria-label={t("settings.providers.registered", "Registered models")}>
      {shown.map((model) => (
        <li key={model.id}>
          <Badge variant="outline" className="gap-1.5 font-normal" title={model.id}>
            <span className="font-mono text-[11px]">{shortModelId(model.id)}</span>
            {capabilitiesOf(model).map((capability) => (
              <span key={capability} className="text-[10px] text-muted-foreground">
                {t(`settings.providers.cap.${capability}`, capability)}
              </span>
            ))}
          </Badge>
        </li>
      ))}
      {row.models.length > MODELS_SHOWN ? (
        <li>
          <Button type="button" variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => setAll((open) => !open)}>
            {all
              ? t("settings.providers.showLess", "Show less")
              : t("settings.providers.showMore", "+{{count}} more", { count: row.models.length - MODELS_SHOWN })}
          </Button>
        </li>
      ) : null}
    </ul>
  );
}

export function ProviderCard({
  row,
  busy,
  onToggleWithdraw,
  actions,
  editor,
}: {
  row: DeploymentProvider;
  busy: boolean;
  onToggleWithdraw: () => void;
  actions?: ReactNode;
  editor?: ReactNode;
}) {
  const { t } = useTranslation();
  return (
    <li
      className="space-y-3 rounded-xl border bg-card p-4 shadow-xs"
      data-testid={`deployment-provider-${row.slug}`}
    >
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 space-y-1">
          <h3 className="truncate text-base font-medium">{row.display_name}</h3>
          <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
            <span>{row.api_shape}</span>
            <span className="inline-flex items-center gap-1" data-testid={`key-${row.slug}`}>
              {row.key_configured ? (
                <>
                  <Check className="h-3.5 w-3.5 text-emerald-600" aria-hidden />
                  {t("settings.providers.keySet", "Key set")}
                </>
              ) : (
                <>
                  <KeyRound className="h-3.5 w-3.5" aria-hidden />
                  {t("settings.providers.noKey", "No key")}
                </>
              )}
            </span>
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {actions}
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={busy}
            data-testid={`withdraw-${row.slug}`}
            onClick={onToggleWithdraw}
          >
            {row.withdrawn
              ? t("settings.providers.restore", "Restore")
              : t("settings.providers.withdraw", "Withdraw")}
          </Button>
        </div>
      </div>
      <StatusBadges row={row} />
      <RefreshLine row={row} />
      {editor ?? <ModelChips row={row} />}
    </li>
  );
}
