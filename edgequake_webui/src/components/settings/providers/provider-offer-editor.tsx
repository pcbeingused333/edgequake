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
  return (
    <Card data-testid="deployment-providers-card">
      <CardContent className="space-y-3 pt-6">
        {query.isPending ? (
          <p className="text-sm text-muted-foreground" aria-busy="true">Loading providers…</p>
        ) : query.isError ? (
          <p className="text-sm text-destructive" role="alert">Could not load providers.</p>
        ) : rows.length === 0 ? (
          <p className="text-sm text-muted-foreground">This deployment has no providers yet. Add one under Connections.</p>
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

function FleetModels({ slug, initial, enabled }: { slug: string; initial: string[]; enabled: boolean }) {
  const save = useSaveConnectionModels(slug);
  const [models, setModels] = useState(initial);
  const [draft, setDraft] = useState("");
  const fieldId = `${slug}-model`;
  return (
    <div className="flex w-full flex-col gap-2 sm:w-72" data-testid={`fleet-models-${slug}`}>
      <div className="flex flex-wrap gap-1">
        {models.map((id) => (
          <Badge key={id} variant="secondary" className="font-normal">{id}</Badge>
        ))}
      </div>
      <div className="flex gap-2">
        <Input
          id={fieldId}
          aria-label={`Model id for ${slug}`}
          value={draft}
          disabled={!enabled || save.isPending}
          placeholder={enabled ? "Model id" : "Test the connection first"}
          onChange={(event) => setDraft(event.target.value)}
        />
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled={!enabled || save.isPending || draft.trim() === ""}
          onClick={() => {
            const id = draft.trim();
            if (!id || models.includes(id)) return;
            const next = [...models, id];
            setModels(next);
            setDraft("");
            void saveWithToast(() => save.mutateAsync(next), "Models saved");
          }}
        >
          Add
        </Button>
      </div>
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
