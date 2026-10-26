/**
 * Embedding model selector for workspace configuration (SPEC-043 two-step picker).
 */
'use client';

import { mergePickerOptions, pairInCatalog } from '@/components/models/model-picker-options';
import { ModelsStatus } from '@/components/models/models-status';
import {
  ModelPickerPanel,
  formatModelFullId,
  type ModelPickerOption,
  type ModelPickerValue,
} from '@/components/models/model-picker-panel';
import { useScopedEligibleModels } from '@/hooks/use-model-policy';
import type { EligibleModel } from '@/lib/api/edgequake/model-catalog';
import { cn } from '@/lib/utils';
import { useTenantStore } from '@/stores/use-tenant-store';
import { Brain } from 'lucide-react';

export interface EmbeddingSelection {
  model: string;
  provider: string;
  dimension: number;
}

interface EmbeddingModelSelectorProps {
  value?: EmbeddingSelection;
  onChange?: (selection: EmbeddingSelection | undefined) => void;
  disabled?: boolean;
  className?: string;
  /** Layout hook for the provider + model panel (e.g. side by side in wide forms). */
  panelClassName?: string;
  /** @deprecated Ignored — provider is a dedicated select. */
  showProviderFilters?: boolean;
  /** LAW-101-13 inherit chip; overrides fleet catalog default label. */
  inheritLabel?: string;
  /** Inherited pair. The "use default" link is hidden when this pair is not eligible. */
  inheritPair?: { provider?: string; model?: string };
  choiceScope?: 'workspace' | 'tenant' | 'deployment';
}

function toOption(model: EligibleModel): ModelPickerOption {
  return {
    provider: model.provider,
    providerDisplayName: model.provider_display_name || model.provider,
    name: model.model,
    displayName: model.display_name || model.model,
    fullId: formatModelFullId(model.provider, model.model),
    contextLength: 0,
    supportsVision: false,
    supportsTools: false,
    supportsStreaming: false,
    dimension: model.embedding_dimension ?? undefined,
    available: true,
  };
}

export function EmbeddingModelSelector({
  value,
  onChange,
  disabled,
  className,
  panelClassName,
  inheritLabel,
  inheritPair,
  choiceScope = 'workspace',
}: EmbeddingModelSelectorProps) {
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const id = choiceScope === 'tenant' ? tenantId : choiceScope === 'workspace' ? workspaceId : null;
  const { data, isLoading, error, refetch } = useScopedEligibleModels({
    scope: choiceScope,
    id,
    usage: 'embedding',
  });

  if (isLoading) {
    return (
      <div data-testid="embedding-model-selector-loading">
        <ModelsStatus state="loading" className={className} />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div data-testid="embedding-model-selector">
        <ModelsStatus state="error" onRetry={() => void refetch()} className={className} />
      </div>
    );
  }

  const options = mergePickerOptions(data.models.map(toOption));

  const pickerValue: ModelPickerValue | undefined = value
    ? {
        provider: value.provider,
        model: value.model,
        fullId: `${value.provider}/${value.model}`,
      }
    : undefined;

  const defaultLabel = inheritLabel ?? 'Inherit';

  const handleChange = (v: ModelPickerValue | undefined) => {
    if (!v) {
      onChange?.(undefined);
      return;
    }
    const match = data.models.find((m) => m.provider === v.provider && m.model === v.model);
    onChange?.({
      provider: v.provider,
      model: v.model,
      dimension: match?.embedding_dimension ?? 0,
    });
  };

  return (
    <div className={cn('space-y-1', className)} data-testid="embedding-model-selector">
      <ModelPickerPanel
        className={panelClassName}
        variant="embedding"
        options={options}
        value={pickerValue}
        onChange={handleChange}
        disabled={disabled}
        serverDefaultLabel={defaultLabel}
        allowServerDefault={
          !inheritPair?.provider ||
          !inheritPair.model ||
          pairInCatalog(inheritPair.provider, inheritPair.model, options)
        }
        placeholder="Search embedding models…"
        testId="embedding-model-picker-panel"
      />
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Brain className="h-3 w-3" />
        <span>Used for vector embeddings of document chunks</span>
      </div>
    </div>
  );
}

export { parseModelFullId as parseFullId } from '@/components/models/model-picker-panel';
