'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { apiClient } from '@/lib/api/client';
import { connectionLocality } from '@/components/settings/connection-locality';
import { connectionSummary } from '@/lib/settings/connection-summary';
import { cn } from '@/lib/utils';
import { PlugZap } from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useId, useState, type ReactNode } from 'react';
import { toast } from 'sonner';

const CONNECTIONS_KEY = ['connections'] as const;

interface ConnectionView {
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

interface ConnectionShape {
  id: string;
  label: string;
  default_base_url: string;
  hint: string;
}

/** Used when the shapes endpoint is unreachable, so the form still offers the two custom shapes. */
const FALLBACK_SHAPES: ConnectionShape[] = [
  { id: 'openai_chat', label: 'OpenAI-compatible', default_base_url: 'https://api.openai.com/v1', hint: 'Chat completions API.' },
  { id: 'anthropic_messages', label: 'Anthropic-compatible', default_base_url: 'https://api.anthropic.com', hint: 'Messages API.' },
  { id: 'ollama', label: 'Ollama', default_base_url: 'http://127.0.0.1:11434', hint: 'Local Ollama. A key is optional.' },
];

function storedIn(row: ConnectionView): string | null {
  if (row.source === 'env') return null;
  if (row.secret_backend === 'infisical') return 'Stored in Infisical';
  if (row.secret_backend === 'legacy') return 'Stored with the server key';
  if (row.key_configured) return 'Stored in the database';
  return null;
}

interface ProbeResponse {
  ok: boolean;
  kind: string;
  message: string;
  latency_ms: number;
  embedding_dimension?: number | null;
}

/** A visible label, optional hint, and the input. Keeps the form markup uniform. */
function ConnectionField({
  id,
  label,
  hint,
  className,
  children,
}: {
  id: string;
  label: string;
  hint?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn('space-y-1.5', className)}>
      <Label htmlFor={id}>{label}</Label>
      {children}
      {hint ? <p className="text-xs text-muted-foreground">{hint}</p> : null}
    </div>
  );
}

export function ConnectionsCard() {
  const fieldId = useId();
  const queryClient = useQueryClient();
  const connections = useQuery({
    queryKey: CONNECTIONS_KEY,
    queryFn: () => apiClient<ConnectionView[]>('/connections'),
    select: (data) => (Array.isArray(data) ? data : []),
    retry: false,
  });
  const rows = connections.data ?? [];
  const shapesQuery = useQuery({
    queryKey: ['connection-shapes'],
    queryFn: () => apiClient<ConnectionShape[]>('/connections/shapes'),
    retry: false,
  });
  const shapes = shapesQuery.data?.length ? shapesQuery.data : FALLBACK_SHAPES;
  const [shape, setShape] = useState('openai_chat');
  const [baseUrl, setBaseUrl] = useState('https://api.openai.com/v1');
  const [slug, setSlug] = useState('local');
  const [apiKey, setApiKey] = useState('');
  const [busy, setBusy] = useState(false);

  const refresh = () => queryClient.invalidateQueries({ queryKey: CONNECTIONS_KEY });

  const chooseShape = (next: string) => {
    const previous = shapes.find((item) => item.id === shape);
    const chosen = shapes.find((item) => item.id === next);
    setShape(next);
    if (chosen && (baseUrl.trim() === '' || baseUrl === previous?.default_base_url)) {
      setBaseUrl(chosen.default_base_url);
    }
  };

  const testStored = async (id: string) => {
    setBusy(true);
    try {
      const result = await apiClient<ProbeResponse>(`/connections/${id}/test`, { method: 'POST' });
      if (result.ok) toast.success(`Connected (${result.latency_ms} ms)`);
      else toast.error(result.kind || 'Test failed', { description: result.message });
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Test failed');
    } finally {
      setBusy(false);
    }
  };

  const testDraft = async () => {
    setBusy(true);
    try {
      const place = connectionLocality(baseUrl);
      const result = await apiClient<ProbeResponse>('/providers/test', {
        method: 'POST',
        body: JSON.stringify({
          shape,
          base_url: baseUrl,
          api_key: apiKey || undefined,
          allow_private_network: place.allowPrivate,
        }),
      });
      if (result.ok) {
        toast.success(`Connected (${result.latency_ms} ms)`, {
          description: result.embedding_dimension
            ? `Embedding dimension ${result.embedding_dimension}`
            : result.message,
        });
      } else {
        toast.error(result.kind, { description: result.message });
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Test failed');
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    try {
      const place = connectionLocality(baseUrl);
      await apiClient('/connections', {
        method: 'POST',
        body: JSON.stringify({
          slug,
          display_name: slug,
          api_shape: shape,
          base_url: baseUrl,
          api_key: apiKey || undefined,
          locality: place.locality,
          allow_private_network: place.allowPrivate,
        }),
      });
      setApiKey('');
      toast.success('Connection saved (key write-only)');
      await refresh();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Save failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <PlugZap className="h-5 w-5" />
          LLM connections
        </CardTitle>
        <CardDescription>
          Point EdgeQuake at OpenAI, Anthropic, Ollama, oMLX, MLX-LM, llama.cpp, or any
          OpenAI-/Anthropic-shaped local server. Keys are encrypted at rest and never shown again.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 md:grid-cols-2">
          <ConnectionField id={`${fieldId}-name`} label="Name" hint="Short identifier, for example local">
            <Input
              id={`${fieldId}-name`}
              value={slug}
              onChange={(e) => setSlug(e.target.value)}
              placeholder="local"
            />
          </ConnectionField>
          <ConnectionField id={`${fieldId}-shape`} label="API shape" hint={shapes.find((item) => item.id === shape)?.hint ?? 'How the server speaks'}>
            <select
              id={`${fieldId}-shape`}
              data-testid="connection-shape"
              className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm"
              value={shape}
              onChange={(e) => chooseShape(e.target.value)}
            >
              {shapes.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                </option>
              ))}
            </select>
          </ConnectionField>
          <ConnectionField id={`${fieldId}-url`} label="Base URL" className="md:col-span-2">
            <Input
              id={`${fieldId}-url`}
              value={baseUrl}
              onChange={(e) => setBaseUrl(e.target.value)}
              placeholder="http://127.0.0.1:9050"
            />
          </ConnectionField>
          <ConnectionField
            id={`${fieldId}-key`}
            label="API key"
            hint="Optional. Encrypted at rest and never shown again."
            className="md:col-span-2"
          >
            <Input
              id={`${fieldId}-key`}
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Leave empty for local servers"
              autoComplete="off"
            />
          </ConnectionField>
        </div>
        <div className="flex gap-2">
          <Button type="button" variant="secondary" disabled={busy} onClick={() => void testDraft()}>
            Test connection
          </Button>
          <Button type="button" disabled={busy} onClick={() => void save()}>
            Save
          </Button>
        </div>
        <ul className="space-y-1 text-sm text-muted-foreground">
          {rows.map((r) => (
            <li key={`${r.source}-${r.id}-${r.slug}`} className="flex items-start justify-between gap-3">
              <span className="flex min-w-0 flex-col">
                <span className="font-medium text-foreground">{r.display_name}</span>
                <span className="text-xs text-muted-foreground break-all">{connectionSummary(r)}</span>
                {storedIn(r) ? (
                  <span className="text-xs text-muted-foreground" data-testid="connection-stored-in">
                    {storedIn(r)}
                  </span>
                ) : null}
              </span>
              {r.source !== 'env' && r.id !== '00000000-0000-0000-0000-000000000000' ? (
                <Button type="button" variant="outline" size="sm" disabled={busy} onClick={() => void testStored(r.id)}>
                  Test
                </Button>
              ) : null}
            </li>
          ))}
          {connections.isPending ? (
            <li className="h-5" aria-busy="true">Loading connections…</li>
          ) : connections.isError ? (
            <li role="alert" className="text-destructive">Could not load connections. Refresh the page to try again.</li>
          ) : rows.length === 0 ? (
            <li>No saved connections. Env providers appear here when set.</li>
          ) : null}
        </ul>
      </CardContent>
    </Card>
  );
}
