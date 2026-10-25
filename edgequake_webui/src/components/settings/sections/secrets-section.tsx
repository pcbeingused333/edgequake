"use client";

/**
 * SPEC-176 Organization / Secrets. The database backend is the default.
 * Infisical is optional and is not required to save provider keys.
 */

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SettingsFrame, SettingsNotice } from "@/components/settings/shell/settings-frame";
import { apiClient } from "@/lib/api/client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { KeyRound } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

interface InfisicalView {
  base_url: string;
  client_id: string;
  project_id: string;
  environment: string;
  secret_path: string;
  allow_private_network: boolean;
  client_secret_configured: boolean;
}

interface BackendView {
  kind: "database" | "infisical" | string;
  secrets_key_configured: boolean;
  infisical: InfisicalView | null;
}

const QUERY_KEY = ["secret-backend"] as const;

export function SecretsSection() {
  const { t } = useTranslation();
  return (
    <SettingsFrame
      title={t("settings.shell.sections.secrets", "Secrets")}
      lead={t(
        "settings.shell.secrets.lead",
        "Where provider keys are stored. The database is the default. Infisical is optional.",
      )}
    >
      <SecretBackendCard />
    </SettingsFrame>
  );
}

function SecretBackendCard() {
  const queryClient = useQueryClient();
  const query = useQuery({
    queryKey: QUERY_KEY,
    queryFn: () => apiClient<BackendView>("/secret-backend"),
    retry: false,
  });
  const saved = query.data;

  const [kind, setKind] = useState<string | null>(null);
  const [baseUrl, setBaseUrl] = useState("");
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [projectId, setProjectId] = useState("");
  const [environment, setEnvironment] = useState("");
  const [secretPath, setSecretPath] = useState("/");
  const [allowPrivate, setAllowPrivate] = useState(false);
  const [hydratedFrom, setHydratedFrom] = useState<BackendView | null>(null);

  if (saved && saved !== hydratedFrom) {
    setHydratedFrom(saved);
    setKind(saved.kind);
    setBaseUrl(saved.infisical?.base_url ?? "");
    setClientId(saved.infisical?.client_id ?? "");
    setProjectId(saved.infisical?.project_id ?? "");
    setEnvironment(saved.infisical?.environment ?? "");
    setSecretPath(saved.infisical?.secret_path ?? "/");
    setAllowPrivate(saved.infisical?.allow_private_network ?? false);
    setClientSecret("");
  }

  const selected = kind ?? saved?.kind ?? "database";

  const save = useMutation({
    mutationFn: () =>
      apiClient<BackendView>("/secret-backend", {
        method: "PUT",
        body: JSON.stringify({
          kind: selected,
          base_url: baseUrl || undefined,
          client_id: clientId || undefined,
          client_secret: clientSecret || undefined,
          project_id: projectId || undefined,
          environment: environment || undefined,
          secret_path: secretPath || "/",
          allow_private_network: allowPrivate,
        }),
      }),
    onSuccess: (view) => {
      setClientSecret("");
      queryClient.setQueryData(QUERY_KEY, view);
      toast.success(view.kind === "infisical" ? "Infisical is the secret backend" : "Keys stay in the database");
    },
    onError: (err) => toast.error(err instanceof Error ? err.message : "Could not save the secret backend"),
  });

  const test = useMutation({
    mutationFn: () => apiClient<{ ok: boolean }>("/secret-backend/test", { method: "POST" }),
    onSuccess: () => toast.success("Secret backend accepted a test secret"),
    onError: (err) => toast.error(err instanceof Error ? err.message : "Secret backend test failed"),
  });

  return (
    <Card data-testid="secret-backend-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <KeyRound className="h-5 w-5" />
          Secret backend
        </CardTitle>
        <CardDescription>
          Provider keys are encrypted in the database unless this organization chooses Infisical.
          EdgeQuake starts and serves with neither Infisical nor a master key. Saving a key needs the master key.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {query.isPending ? <p className="text-sm text-muted-foreground">Loading secret backend…</p> : null}
        {query.isError ? (
          <SettingsNotice tone="danger" testId="secret-backend-error">
            Could not load the secret backend.
          </SettingsNotice>
        ) : null}
        {saved && !saved.secrets_key_configured ? (
          <SettingsNotice tone="warning" testId="secrets-key-missing">
            EDGEQUAKE_SECRETS_KEY is not set. The server is running. Saving a provider key is refused until the key is set.
          </SettingsNotice>
        ) : null}

        <fieldset className="space-y-2">
          <legend className="text-sm font-medium">Where new keys are written</legend>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name="secret-backend"
              value="database"
              checked={selected === "database"}
              onChange={() => setKind("database")}
            />
            Database (default)
          </label>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="radio"
              name="secret-backend"
              value="infisical"
              data-testid="secret-backend-infisical"
              checked={selected === "infisical"}
              onChange={() => setKind("infisical")}
            />
            Infisical (optional)
          </label>
        </fieldset>

        {selected === "infisical" ? (
          <div className="grid gap-3 md:grid-cols-2" data-testid="infisical-form">
            <Field id="infisical-url" label="Base URL" value={baseUrl} onChange={setBaseUrl} placeholder="https://us.infisical.com" />
            <Field id="infisical-client" label="Client ID" value={clientId} onChange={setClientId} />
            <Field
              id="infisical-secret"
              label="Client secret"
              value={clientSecret}
              onChange={setClientSecret}
              type="password"
              hint={saved?.infisical?.client_secret_configured ? "A secret is already stored. Leave this blank to keep it." : "Write-only. Encrypted in the database."}
              className="md:col-span-2"
            />
            <Field id="infisical-project" label="Project ID" value={projectId} onChange={setProjectId} />
            <Field id="infisical-env" label="Environment" value={environment} onChange={setEnvironment} placeholder="dev" />
            <Field id="infisical-path" label="Secret path" value={secretPath} onChange={setSecretPath} placeholder="/" />
            <label className="flex items-center gap-2 text-sm md:col-span-2">
              <input type="checkbox" checked={allowPrivate} onChange={(e) => setAllowPrivate(e.target.checked)} />
              Allow a private or loopback Infisical URL
            </label>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">
            Keys are encrypted with EDGEQUAKE_SECRETS_KEY and stored in this database. No Infisical account is required.
          </p>
        )}

        <div className="flex gap-2">
          <Button type="button" disabled={save.isPending} onClick={() => save.mutate()} data-testid="secret-backend-save">
            Save
          </Button>
          <Button type="button" variant="secondary" disabled={test.isPending} onClick={() => test.mutate()}>
            Test
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

function Field({
  id,
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  hint,
  className,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: string;
  hint?: string;
  className?: string;
}) {
  return (
    <div className={className ? `space-y-1.5 ${className}` : "space-y-1.5"}>
      <Label htmlFor={id}>{label}</Label>
      <Input id={id} type={type} value={value} placeholder={placeholder} autoComplete="off" onChange={(e) => onChange(e.target.value)} />
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}
