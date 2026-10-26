'use client';

import { OptionCard } from '@/components/onboarding/option-card';
import { Skeleton } from '@/components/ui/skeleton';
import { useServerModelDefaults } from '@/hooks/use-server-model-defaults';
import type {
  DefaultsSource,
  ModelDefaultsSlice,
} from '@/lib/onboarding/inherited-defaults';
import { cn } from '@/lib/utils';
import {
  BLOCKING,
  knownEmbeddingDimension,
  type RoleStatus,
  type WizardRole,
} from '@/lib/onboarding/defaults-eligibility';
import { useTranslation } from 'react-i18next';

export interface ResolvedDefaultsDisplay extends ModelDefaultsSlice {
  isLoading?: boolean;
  hasConfiguredDefaults: boolean;
  /** When set, drives title (“Using organization defaults” / “Using server defaults”). */
  source?: DefaultsSource;
  sourceLabel?: string;
  sourceHint?: string;
  /** SPEC-179: how each default fares against the offer. */
  verdicts?: Partial<Record<WizardRole, RoleStatus>>;
}

export interface ServerDefaultsCardProps {
  className?: string;
  /** Radio state: true when the workspace follows these defaults. */
  selected?: boolean;
  /** Choose “use the defaults”. */
  onSelect?: () => void;
  /**
   * When provided, render these resolved lines instead of calling useServerModelDefaults.
   * Used by Create Workspace (tenant → server ladder).
   */
  defaults?: ResolvedDefaultsDisplay;
}

function formatId(provider?: string, model?: string): string {
  if (provider && model) return `${provider}/${model}`;
  if (model) return model;
  return 'not configured';
}

function VerdictBadge({ status }: { status?: RoleStatus }) {
  const { t } = useTranslation();
  if (status !== 'not_eligible' && status !== 'dimension_missing') return null;
  return (
    <span
      className="rounded-md bg-destructive/10 px-1.5 py-0.5 font-sans text-[10px] font-medium text-destructive"
      data-testid="server-defaults-verdict"
    >
      {status === 'dimension_missing'
        ? t('onboarding.eligibility.badgeNoSize', 'No vector size')
        : t('onboarding.eligibility.badgeNotOffered', 'Not offered')}
    </span>
  );
}

function DefaultRow({
  role,
  testId,
  value,
  detail,
  status,
}: {
  role: string;
  testId: string;
  value: string;
  detail?: string;
  status?: RoleStatus;
}) {
  return (
    <div
      className="grid grid-cols-[4.5rem_minmax(0,1fr)] items-baseline gap-2"
      data-testid={testId}
    >
      <dt className="text-xs font-medium text-muted-foreground">{role}</dt>
      <dd className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-xs">
        <span className="break-all">{value}</span>
        {detail ? (
          <span className="font-sans text-[11px] text-muted-foreground">
            {detail}
          </span>
        ) : null}
        <VerdictBadge status={status} />
      </dd>
    </div>
  );
}

function ServerDefaultsCardView({
  className,
  selected = true,
  onSelect,
  isLoading,
  hasConfiguredDefaults,
  defaultLlmProvider,
  defaultLlmModel,
  defaultEmbeddingProvider,
  defaultEmbeddingModel,
  defaultEmbeddingDimension,
  defaultVisionProvider,
  defaultVisionModel,
  source = 'server',
  sourceLabel,
  sourceHint,
  verdicts,
}: ServerDefaultsCardProps &
  ResolvedDefaultsDisplay & {
    isLoading: boolean;
  }) {
  const { t } = useTranslation();

  const title =
    sourceLabel ??
    (source === 'tenant'
      ? t('onboarding.useTenantDefaults', 'Use organization defaults')
      : hasConfiguredDefaults
        ? t('onboarding.useServerDefaults', 'Use server defaults')
        : t(
            'onboarding.serverDefaultsMissing',
            'Defaults not fully configured',
          ));

  const hint =
    sourceHint ??
    (source === 'tenant'
      ? t(
          'onboarding.tenantDefaultsHint',
          'Inherited from your organization, then the server.',
        )
      : t('onboarding.serverDefaultsHint', 'Set by the server configuration.'));

  if (isLoading) {
    return (
      <div
        className={cn('space-y-2 rounded-xl border p-3.5', className)}
        data-testid="server-defaults-card-loading"
      >
        <Skeleton className="h-4 w-40" />
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-3/4" />
      </div>
    );
  }

  const refused = Object.values(verdicts ?? {}).some((status) => BLOCKING.includes(status));
  const dimension =
    defaultEmbeddingDimension ||
    knownEmbeddingDimension(defaultEmbeddingModel ?? '');

  return (
    <div
      className={className}
      data-testid="server-defaults-card"
      data-configured={hasConfiguredDefaults ? 'true' : 'false'}
      data-overridden={selected ? 'false' : 'true'}
      data-source={source}
    >
      <OptionCard
        className="h-full"
        selected={selected}
        onSelect={() => onSelect?.()}
        disabled={!hasConfiguredDefaults || !onSelect}
        title={title}
        badge={
          hasConfiguredDefaults && !refused
            ? t('onboarding.recommended', 'Recommended')
            : undefined
        }
        data-testid="wizard-models-use-defaults"
      >
        <dl className="space-y-1.5">
          <DefaultRow
            role="LLM"
            testId="server-defaults-llm"
            value={formatId(defaultLlmProvider, defaultLlmModel)}
            status={verdicts?.llm}
          />
          <DefaultRow
            role="Embedding"
            testId="server-defaults-embedding"
            value={formatId(defaultEmbeddingProvider, defaultEmbeddingModel)}
            detail={
              dimension > 0
                ? t('onboarding.dimensions', '{{dimension}} dims', {
                    dimension: String(dimension),
                  })
                : undefined
            }
            status={verdicts?.embedding}
          />
          <DefaultRow
            role="Vision"
            testId="server-defaults-vision"
            value={formatId(defaultVisionProvider, defaultVisionModel)}
            status={verdicts?.vision}
          />
        </dl>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </OptionCard>
    </div>
  );
}

function ServerDefaultsCardFromServer(
  props: Omit<ServerDefaultsCardProps, 'defaults'>,
) {
  const server = useServerModelDefaults();
  return (
    <ServerDefaultsCardView
      {...props}
      isLoading={server.isLoading}
      hasConfiguredDefaults={server.hasConfiguredDefaults}
      defaultLlmProvider={server.defaultLlmProvider}
      defaultLlmModel={server.defaultLlmModel}
      defaultEmbeddingProvider={server.defaultEmbeddingProvider}
      defaultEmbeddingModel={server.defaultEmbeddingModel}
      defaultVisionProvider={server.defaultVisionProvider}
      defaultVisionModel={server.defaultVisionModel}
      source="server"
    />
  );
}

/**
 * SPEC-101 — Explicit defaults for LLM · Embedding · Vision (LAW-101-2).
 * Presentational when `defaults` prop is set; otherwise loads server defaults.
 */
export function ServerDefaultsCard({
  defaults,
  ...rest
}: ServerDefaultsCardProps) {
  if (defaults) {
    return (
      <ServerDefaultsCardView
        {...rest}
        isLoading={Boolean(defaults.isLoading)}
        hasConfiguredDefaults={defaults.hasConfiguredDefaults}
        defaultLlmProvider={defaults.defaultLlmProvider}
        defaultLlmModel={defaults.defaultLlmModel}
        defaultEmbeddingProvider={defaults.defaultEmbeddingProvider}
        defaultEmbeddingModel={defaults.defaultEmbeddingModel}
        defaultEmbeddingDimension={defaults.defaultEmbeddingDimension}
        defaultVisionProvider={defaults.defaultVisionProvider}
        defaultVisionModel={defaults.defaultVisionModel}
        source={defaults.source ?? 'server'}
        sourceLabel={defaults.sourceLabel}
        sourceHint={defaults.sourceHint}
        verdicts={defaults.verdicts}
      />
    );
  }
  return <ServerDefaultsCardFromServer {...rest} />;
}
