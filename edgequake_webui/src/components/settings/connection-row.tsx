"use client";

/** SPEC-179 — one saved connection: name, state badges, where it points, and its actions. */

import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { connectionSourceLabel } from "@/lib/settings/connection-summary";

export interface ConnectionRowData {
  id: string;
  slug: string;
  display_name: string;
  api_shape: string;
  base_url: string;
  source: string;
  key_configured: boolean;
  key_fingerprint?: string | null;
  secret_backend?: string | null;
  last_test_ok?: boolean | null;
}

const BUILT_IN_ID = "00000000-0000-0000-0000-000000000000";

/** Env and built-in connections are read-only: nothing to update, test or delete here. */
export const isEditableConnection = (row: Pick<ConnectionRowData, "source" | "id">) =>
  row.source !== "env" && row.id !== BUILT_IN_ID;

function storedIn(row: ConnectionRowData): string | null {
  if (row.source === "env") return null;
  if (row.secret_backend === "infisical") return "Stored in Infisical";
  if (row.secret_backend === "legacy") return "Stored with the server key";
  if (row.key_configured) return "Stored in the database";
  return null;
}

function TestBadge({ ok }: { ok: boolean | null | undefined }) {
  const { t } = useTranslation();
  if (ok === undefined || ok === null) return null;
  return (
    <Badge
      variant="outline"
      className={cn(
        "font-normal",
        ok
          ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
          : "border-destructive/40 bg-destructive/10 text-destructive",
      )}
    >
      {ok ? t("settings.connections.testOk", "Last test passed") : t("settings.connections.testFailed", "Last test failed")}
    </Badge>
  );
}

export function ConnectionRow({
  row,
  busy,
  onUpdate,
  onTest,
  onDelete,
}: {
  row: ConnectionRowData;
  busy: boolean;
  onUpdate: () => void;
  onTest: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation();
  const stored = storedIn(row);
  return (
    <li
      className="flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between"
      data-testid={`connection-row-${row.slug}`}
    >
      <div className="min-w-0 space-y-1.5">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-medium text-foreground">{row.display_name}</span>
          <Badge variant="outline" className="font-normal">
            {connectionSourceLabel(row.source)}
          </Badge>
          {row.key_configured ? (
            <Badge variant="outline" className="font-mono text-[11px] font-normal">
              {t("settings.connections.key", "key")} {row.key_fingerprint ?? t("settings.connections.keySet", "set")}
            </Badge>
          ) : null}
          <TestBadge ok={row.last_test_ok} />
        </div>
        <p className="truncate text-xs text-muted-foreground" title={`${row.api_shape} · ${row.base_url}`}>
          {row.api_shape} · {row.base_url}
        </p>
        {stored ? (
          <p className="text-xs text-muted-foreground" data-testid="connection-stored-in">
            {stored}
          </p>
        ) : null}
      </div>
      {isEditableConnection(row) ? (
        <div className="flex shrink-0 gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy}
            data-testid={`connection-update-${row.slug}`}
            onClick={onUpdate}
          >
            {t("settings.connections.update", "Update")}
          </Button>
          <Button type="button" variant="outline" size="sm" disabled={busy} onClick={onTest}>
            {t("settings.connections.test", "Test")}
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy}
            data-testid={`connection-delete-${row.slug}`}
            onClick={onDelete}
          >
            {t("settings.connections.delete", "Delete")}
          </Button>
        </div>
      ) : null}
    </li>
  );
}
