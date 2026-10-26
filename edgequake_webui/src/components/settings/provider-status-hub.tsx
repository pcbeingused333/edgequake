"use client";

/**
 * Interactive provider status hub — filter models by provider health (SPEC-043).
 */
import { ProviderIcon } from "@/components/providers/provider-icon";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { getProviderAuthLabel, getProviderDisplayName } from "@/lib/provider-display";
import type { ProviderResponse } from "@/lib/api/models";
import { CheckCircle, ChevronDown, ChevronRight, RefreshCw, Server, XCircle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export interface ProviderStatusHubProps {
  providers?: ProviderResponse[];
  isLoading?: boolean;
  onRefresh?: () => void;
  /** Eligible model count per provider for the query usage. Absent until loaded, never inventory. */
  eligibleCounts?: Record<string, number>;
}

export function ProviderStatusHub({
  providers = [],
  isLoading,
  onRefresh,
  eligibleCounts,
}: ProviderStatusHubProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState<string | null>(null);

  // SPEC-100: tolerate non-array payloads during soft-load / mock drift
  const providerList = Array.isArray(providers) ? providers : [];
  const enabled = providerList.filter((p) => p.enabled);

  return (
    <Card data-testid="provider-status-hub">
      <CardHeader>
        <div className="flex items-center justify-between gap-2">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Server className="h-5 w-5" />
              Provider Status
            </CardTitle>
            <CardDescription>
              Green = reachable with configured credentials.
            </CardDescription>
          </div>
          {onRefresh && (
            <Button
              variant="outline"
              size="sm"
              data-testid="provider-status-refresh"
              onClick={onRefresh}
              disabled={isLoading}
            >
              <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-2">
        {isLoading && providerList.length === 0 ? (
          <div className="space-y-2" data-testid="provider-status-skeleton" aria-busy="true">
            {Array.from({ length: 3 }, (_, i) => (
              <div key={i} className="h-14 rounded-lg border bg-muted/40" />
            ))}
          </div>
        ) : enabled.length === 0 ? (
          <p className="text-sm text-muted-foreground">No providers configured.</p>
        ) : (
          enabled.map((provider) => {
            const available = provider.health?.available ?? provider.enabled;
            const isOpen = expanded === provider.name;
            const authLabel = getProviderAuthLabel(provider.name, provider.auth_kind);
            return (
              <div
                key={provider.name}
                data-testid={`provider-status-row-${provider.name}`}
                className="rounded-lg border border-border p-3 transition-colors"
              >
                <button
                  type="button"
                  className="flex w-full items-center justify-between gap-2 text-left"
                  onClick={() => {
                    setExpanded(isOpen ? null : provider.name);
                  }}
                >
                  <div className="flex items-center gap-2 min-w-0 flex-wrap">
                    {isOpen ? (
                      <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
                    )}
                    <ProviderIcon providerId={provider.name} />
                    <span className="font-medium truncate">
                      {provider.display_name || getProviderDisplayName(provider.name)}
                    </span>
                    {authLabel && (
                      <Badge
                        variant="outline"
                        className="text-xs font-normal"
                        data-testid={`provider-auth-badge-${provider.name}`}
                      >
                        {authLabel}
                      </Badge>
                    )}
                    <Badge
                      variant={available ? "default" : "secondary"}
                      className={
                        available
                          ? "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300"
                          : "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300"
                      }
                    >
                      {available ? (
                        <CheckCircle className="h-3 w-3 mr-1 inline" />
                      ) : (
                        <XCircle className="h-3 w-3 mr-1 inline" />
                      )}
                      {available ? "online" : "offline"}
                    </Badge>
                    {eligibleCounts?.[provider.name] !== undefined && (
                      <span
                        className="text-xs text-muted-foreground"
                        data-testid={`provider-status-eligible-${provider.name}`}
                      >
                        {t("settings.fleet.eligibleForQueries", "{{count}} eligible for queries", {
                          count: eligibleCounts[provider.name],
                        })}
                      </span>
                    )}
                  </div>
                </button>
                {isOpen && (
                  <div
                    className="mt-2 pl-6 text-xs text-muted-foreground space-y-1"
                    data-testid={`provider-status-detail-${provider.name}`}
                  >
                    <p>Provider ID: {provider.name}</p>
                    {provider.description && <p>{provider.description}</p>}
                    {provider.name === "vertexai" && (
                      <p>
                        Docs:{" "}
                        <a
                          href="https://cloud.google.com/vertex-ai/docs/authentication"
                          target="_blank"
                          rel="noopener noreferrer"
                          className="underline text-primary"
                        >
                          Vertex AI authentication
                        </a>
                      </p>
                    )}
                    {provider.config_requirements && provider.config_requirements.length > 0 && (
                      <ul
                        className="list-disc pl-4 space-y-0.5"
                        data-testid={`provider-config-requirements-${provider.name}`}
                      >
                        {provider.config_requirements.map((req) => (
                          <li
                            key={req.env_var}
                            className={req.satisfied ? "text-green-700 dark:text-green-400" : undefined}
                          >
                            {req.required ? "Required: " : "Optional: "}
                            {req.description}
                            {req.satisfied ? " ✓" : " — not set"}
                          </li>
                        ))}
                      </ul>
                    )}
                    {provider.health?.error && (
                      <p className="text-destructive" data-testid={`provider-health-error-${provider.name}`}>
                        {provider.health.error}
                      </p>
                    )}
                  </div>
                )}
              </div>
            );
          })
        )}
      </CardContent>
    </Card>
  );
}
