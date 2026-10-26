"use client";

/**
 * SPEC-177 — one provider editor. The parent argument selects the payload.
 * Model ticks for a provider that is off are not part of the offer save.
 */

import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { ModelSaveBar, saveWithToast } from "@/components/settings/models/model-save-bar";
import { useServerBackedDraft } from "@/hooks/use-server-backed-draft";
import { refreshDynamicModels, useAvailableProviders } from "@/hooks/use-providers";
import { apiClient } from "@/lib/api/client";
import type { LlmModelItem, LlmModelsResponse } from "@/lib/api/models";
import {
  connectionModelBody,
  showFleetModelRefresh,
  showProviderModelRefresh,
  toggleModelId,
} from "@/lib/settings/fleet-model-refresh";
import { sectionHref } from "@/lib/settings/registry";
import Link from "next/link";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import {
  useDeploymentProviders,
  useSaveConnectionModels,
  useSaveTenantOffer,
  useSaveWorkspaceOffer,
  useTenantOffer,
  useWithdrawProvider,
  useWorkspaceOffer,
} from "@/hooks/use-provider-offer";
import type { ProviderOffer } from "@/lib/api/edgequake/provider-offer";
import {
  draftFromProviders,
  lead,
  sameDraft,
  slugsForSave,
  toggleSlug,
  type OfferDraft,
} from "@/lib/settings/provider-offer";

const EMPTY: ProviderOffer = {
  tenant_id: "",
  offer_mode: "unconstrained",
  parent_count: 0,
  enabled_count: 0,
  can_manage: false,
  providers: [],
  legacy: [],
};

function toDraft(offer: ProviderOffer): OfferDraft {
  return draftFromProviders(
    offer.offer_mode ?? "unconstrained",
    (offer.providers ?? []).map((row) => ({ slug: row.slug, enabled: row.enabled })),
  );
}

interface GrantEditorProps {
  parent: "tenant" | "workspace";
  offer: ProviderOffer | undefined;
  pending: boolean;
  readOnly: boolean;
  onSave: (body: { offer_mode: "unconstrained" | "constrained"; slugs: string[] }) => Promise<unknown>;
}

function GrantEditor({ parent, offer, pending, readOnly, onSave }: GrantEditorProps) {
  const saved = offer?.providers ? offer : EMPTY;
  const { draft, setDraft, reset } = useServerBackedDraft(saved, toDraft);
  const dirty = !sameDraft(draft, toDraft(saved));
  const parentSlugs = (saved.providers ?? []).map((row) => row.slug);
  const enabled = new Set(draft.mode === "unconstrained" ? parentSlugs : draft.slugs);
  const confirm = useStateConfirm();

  const count = enabled.size;
  const sentence =
    saved.providers.length === 0
      ? "This deployment has no providers yet. Add one under Connections."
      : lead(parent, draft.mode);

  const save = (mode: OfferDraft["mode"], slugs: string[]) =>
    saveWithToast(() => onSave({ offer_mode: mode, slugs }), "Providers saved");

  const requestSave = () => {
    if (saved.offer_mode === "unconstrained" && draft.mode === "constrained") {
      confirm.open(
        parent === "tenant"
          ? "From now on, only providers you enable here appear in workspaces."
          : "From now on, only providers you enable here appear in this workspace.",
        () => {
          void save("constrained", slugsForSave(draft));
        },
      );
      return;
    }
    void save(draft.mode, slugsForSave(draft));
  };

  return (
    <Card data-testid="provider-offer-card">
      <CardHeader className="pb-2">
        <CardDescription data-testid="provider-offer-lead">{sentence}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {pending ? (
          <p className="text-sm text-muted-foreground" aria-busy="true">Loading providers…</p>
        ) : (saved.providers ?? []).length === 0 ? null : (
          <>
            <p className="text-sm text-muted-foreground" data-testid="provider-offer-count">
              {count} of {saved.parent_count} providers
            </p>
            <ul className="divide-y rounded-lg border">
              {(saved.providers ?? []).map((row) => {
                const on = enabled.has(row.slug);
                return (
                  <li key={row.slug} className="flex flex-col gap-2 px-3 py-3 sm:flex-row sm:items-center sm:justify-between" data-testid={`provider-row-${row.slug}`}>
                    <label className="flex items-start gap-3">
                      <Checkbox
                        checked={on}
                        disabled={readOnly || !saved.can_manage}
                        aria-label={`Offer ${row.display_name}`}
                        data-testid={`provider-toggle-${row.slug}`}
                        onCheckedChange={() =>
                          setDraft((current) => toggleSlug(current, row.slug, parentSlugs))
                        }
                      />
                      <span>
                        <span className="block font-medium">{row.display_name}</span>
                        <span className="text-xs text-muted-foreground">
                          {(row.models ?? []).length} {(row.models ?? []).length === 1 ? "model" : "models"}
                        </span>
                      </span>
                    </label>
                    {on && (row.models ?? []).length > 0 ? (
                      <div className="flex flex-wrap gap-1 sm:max-w-md sm:justify-end">
                        {(row.models ?? []).slice(0, 4).map((model) => (
                          <Badge key={model.id} variant="outline" className="font-normal">
                            {model.id}
                          </Badge>
                        ))}
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          </>
        )}
        {parent === "tenant" && (saved.legacy ?? []).length > 0 ? (
          <div className="rounded-lg border border-dashed px-3 py-3 text-sm" data-testid="legacy-connections">
            <p className="font-medium">This organization has a key saved earlier.</p>
            <ul className="mt-2 space-y-1 text-muted-foreground">
              {(saved.legacy ?? []).map((row) => (
                <li key={row.slug}>
                  {row.display_name}. New keys are added on Deployment.
                </li>
              ))}
            </ul>
          </div>
        ) : null}
        {saved.can_manage && !readOnly ? (
          <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
            {draft.mode === "constrained" ? (
              <Button
                type="button"
                variant="ghost"
                className="justify-start px-0"
                onClick={() =>
                  confirm.open(
                    "Workspaces will be able to use every provider this deployment offers, including ones added later.",
                    () => {
                      setDraft(() => ({ mode: "unconstrained", slugs: parentSlugs }));
                      void save("unconstrained", []);
                    },
                  )
                }
              >
                Use every provider the parent offers
              </Button>
            ) : (
              <span />
            )}
            <ModelSaveBar dirty={dirty} saving={false} onSave={requestSave} onDiscard={reset} />
          </div>
        ) : null}
        <AlertDialog open={confirm.state.open} onOpenChange={(open) => !open && confirm.close()}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>Change which providers are offered</AlertDialogTitle>
              <AlertDialogDescription>{confirm.state.message}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={() => {
                  confirm.state.action();
                  confirm.close();
                }}
              >
                Confirm
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardContent>
    </Card>
  );
}

function useStateConfirm() {
  const [state, setState] = useState<{ open: boolean; message: string; action: () => void }>({
    open: false,
    message: "",
    action: () => undefined,
  });
  return {
    state,
    open: (message: string, action: () => void) => setState({ open: true, message, action }),
    close: () => setState((current) => ({ ...current, open: false })),
  };
}

export function ProviderOfferEditor({
  parent,
  tenantId,
  workspaceId,
  readOnly = false,
}: {
  parent: "deployment" | "tenant" | "workspace";
  tenantId?: string | null;
  workspaceId?: string | null;
  readOnly?: boolean;
}) {
  if (parent === "deployment") return <DeploymentProviders />;
  if (parent === "tenant") return <TenantProviders tenantId={tenantId ?? null} readOnly={readOnly} />;
  return <WorkspaceProviders workspaceId={workspaceId ?? null} readOnly={readOnly} />;
}

function DeploymentProviders() {
  const query = useDeploymentProviders();
  const withdraw = useWithdrawProvider();
  const rows = query.data?.providers ?? [];
  const addHref = sectionHref("connections");
  return (
    <Card data-testid="deployment-providers-card">
      <CardContent className="space-y-3 pt-6">
        <div className="flex justify-end">
          <Button type="button" variant="outline" size="sm" asChild>
            <Link href={addHref} data-testid="add-provider-link">Add a provider</Link>
          </Button>
        </div>
        {query.isPending ? (
          <p className="text-sm text-muted-foreground" aria-busy="true">Loading providers…</p>
        ) : query.isError ? (
          <p className="text-sm text-destructive" role="alert">Could not load providers.</p>
        ) : rows.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            This deployment has no providers yet.{" "}
            <Link href={addHref} className="underline">Add one under Connections.</Link>
          </p>
        ) : (
          <ul className="divide-y rounded-lg border">
            {rows.map((row) => {
              const label = row.withdrawn ? "Hidden from organizations" : row.offered ? "Offered" : "Not offered";
              return (
                <li key={row.slug} className="flex flex-col gap-2 px-3 py-3 sm:flex-row sm:items-center sm:justify-between" data-testid={`deployment-provider-${row.slug}`}>
                  <div>
                    <div className="font-medium">{row.display_name}</div>
                    <div className="text-xs text-muted-foreground">{row.api_shape}</div>
                  </div>
                  <div className="flex flex-col items-stretch gap-2 sm:items-end">
                    <div className="flex items-center gap-2">
                      <Badge variant="outline">{label}</Badge>
                      {row.origin === "env" ? (
                        <Button
                          type="button"
                          size="sm"
                          variant="outline"
                          disabled={withdraw.isPending}
                          onClick={() => void withdraw.mutateAsync({ slug: row.slug, withdrawn: !row.withdrawn })}
                        >
                          {row.withdrawn ? "Restore" : "Withdraw"}
                        </Button>
                      ) : null}
                    </div>
                    {row.origin === "env" ? (
                      <EnvModelRefresh slug={row.slug} probe={row.probe} />
                    ) : null}
                    {row.origin === "fleet" ? (
                      <FleetModels slug={row.slug} initial={row.models.map((model) => model.id)} enabled={row.probe === "ok"} />
                    ) : null}
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}

function EnvModelRefresh({ slug, probe }: { slug: string; probe?: string | null }) {
  const providers = useAvailableProviders();
  const queryClient = useQueryClient();
  const refreshable =
    providers.data?.llm_providers.some((row) => row.id === slug && row.refreshable === true) ?? false;
  const [busy, setBusy] = useState(false);
  if (!showProviderModelRefresh({ refreshable, origin: "env", probe })) return null;
  return (
    <Button
      type="button"
      size="sm"
      variant="outline"
      disabled={busy}
      data-testid={`env-refresh-${slug}`}
      onClick={() => {
        setBusy(true);
        void refreshDynamicModels(queryClient)
          .then(() => toast.success("Models refreshed"))
          .catch(() => toast.error("Could not refresh models"))
          .finally(() => setBusy(false));
      }}
    >
      {busy ? "Refreshing…" : "Refresh models"}
    </Button>
  );
}

const MODEL_ID_SEPARATOR = "\u0000";
const savedModelIds = (key: string): string[] => (key ? key.split(MODEL_ID_SEPARATOR) : []);

function FleetModels({ slug, initial, enabled }: { slug: string; initial: string[]; enabled: boolean }) {
  const save = useSaveConnectionModels(slug);
  const providers = useAvailableProviders();
  const queryClient = useQueryClient();
  const refreshable =
    providers.data?.llm_providers.some((row) => row.id === slug && row.refreshable === true) ?? false;
  const canRefresh = showFleetModelRefresh(refreshable, enabled);
  // A string key keeps the draft stable across parent re-renders; it restarts only when the saved ids change.
  const savedKey = initial.join(MODEL_ID_SEPARATOR);
  const { draft: models, setDraft: setModels, reset } = useServerBackedDraft(savedKey, savedModelIds);
  const dirty = models.join(MODEL_ID_SEPARATOR) !== savedKey;
  const [newId, setNewId] = useState("");
  const [discovered, setDiscovered] = useState<LlmModelItem[]>([]);
  const [refreshing, setRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const fieldId = `${slug}-model`;

  const refresh = async () => {
    setRefreshing(true);
    setRefreshError(null);
    try {
      await refreshDynamicModels(queryClient);
      const response = await apiClient<LlmModelsResponse>(
        `/models/llm?provider=${encodeURIComponent(slug)}`,
      );
      setDiscovered(response.models ?? []);
    } catch {
      setRefreshError("Could not refresh models.");
    } finally {
      setRefreshing(false);
    }
  };

  const addId = () => {
    const id = newId.trim();
    if (!id || models.includes(id)) return;
    setModels((current) => [...current, id]);
    setNewId("");
  };

  const saveModels = () =>
    saveWithToast(() => save.mutateAsync(connectionModelBody(models, discovered)), "Models saved");

  return (
    <div className="flex w-full flex-col gap-2 sm:w-72" data-testid={`fleet-models-${slug}`}>
      <div className="flex flex-wrap gap-1">
        {models.map((id) => (
          <Badge key={id} variant="secondary" className="font-normal">{id}</Badge>
        ))}
      </div>
      {canRefresh ? (
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={refreshing || save.isPending}
          data-testid={`fleet-refresh-${slug}`}
          onClick={() => void refresh()}
        >
          {refreshing ? "Refreshing…" : "Refresh models"}
        </Button>
      ) : null}
      {refreshError ? (
        <p className="text-xs text-destructive" data-testid={`fleet-refresh-error-${slug}`}>
          {refreshError}
        </p>
      ) : null}
      {discovered.length > 0 ? (
        <ul className="flex max-h-40 flex-col gap-1 overflow-auto" data-testid={`fleet-discovered-${slug}`}>
          {discovered.map((item) => (
            <li key={item.name} className="flex items-center gap-2 text-sm">
              <Checkbox
                id={`${slug}-${item.name}`}
                checked={models.includes(item.name)}
                disabled={save.isPending}
                onCheckedChange={(checked) =>
                  setModels((current) => toggleModelId(current, item.name, checked === true))
                }
                aria-label={item.display_name || item.name}
              />
              <label htmlFor={`${slug}-${item.name}`} className="min-w-0 truncate">
                {item.display_name || item.name}
              </label>
            </li>
          ))}
        </ul>
      ) : null}
      <div className="flex gap-2">
        <Input
          id={fieldId}
          aria-label={`Model id for ${slug}`}
          value={newId}
          disabled={!enabled || save.isPending}
          placeholder={enabled ? "Model id" : "Test the connection first"}
          onChange={(event) => setNewId(event.target.value)}
        />
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={!enabled || save.isPending || newId.trim() === ""}
          onClick={addId}
        >
          Add
        </Button>
      </div>
      <ModelSaveBar dirty={dirty} saving={save.isPending} onSave={() => void saveModels()} onDiscard={reset} />
    </div>
  );
}

function TenantProviders({ tenantId, readOnly }: { tenantId: string | null; readOnly: boolean }) {
  const query = useTenantOffer(tenantId);
  const save = useSaveTenantOffer(tenantId ?? "");
  if (!tenantId) return null;
  return (
    <GrantEditor
      parent="tenant"
      offer={query.data}
      pending={query.isPending}
      readOnly={readOnly}
      onSave={(body) => save.mutateAsync(body)}
    />
  );
}

function WorkspaceProviders({ workspaceId, readOnly }: { workspaceId: string | null; readOnly: boolean }) {
  const query = useWorkspaceOffer(workspaceId);
  const save = useSaveWorkspaceOffer(workspaceId ?? "");
  if (!workspaceId) return null;
  return (
    <GrantEditor
      parent="workspace"
      offer={query.data}
      pending={query.isPending}
      readOnly={readOnly || query.data?.can_manage === false}
      onSave={(body) => save.mutateAsync(body)}
    />
  );
}
