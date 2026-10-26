'use client';

import { useLlmCapabilityModels } from "@/hooks/use-llm-capability-models";
import { ServerDefaultsCard } from '@/components/onboarding/server-defaults-card';
import { WizardEligibilityNotice } from '@/components/onboarding/wizard-eligibility-notice';
import type { WizardEligibility } from '@/hooks/use-wizard-eligibility';
import type { WizardRole } from '@/lib/onboarding/defaults-eligibility';
import { ReasoningEffortSelect } from '@/components/settings/reasoning-effort-select';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { DecisionEngineFields } from '@/components/workspace/decision-engine-fields';
import {
  EmbeddingModelSelector,
  type EmbeddingSelection,
} from '@/components/workspace/embedding-model-selector';
import {
  LLMModelSelector,
  type LLMSelection,
} from '@/components/workspace/llm-model-selector';
import { useInheritedModelDefaults } from '@/hooks/use-inherited-model-defaults';
import { useDecisionModels, useDecisionStatus } from '@/hooks/use-decision-status';
import { formatInheritModelLabel } from '@/lib/onboarding/inherited-defaults';
import type { WizardDraft } from '@/lib/onboarding/wizard-state';
import {
  effectiveEffortWhenAuto,
  modelSupportsThinking,
  supportedReasoningEffortsForModel,
} from '@/lib/settings/reasoning-effort-supported';
import { draftIssue, type ExtractionModeDraft } from '@/lib/workspace/extraction-mode-draft';
import { useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export interface ModelDefaultsStepProps {
  draft: WizardDraft;
  onChange: (patch: Partial<WizardDraft>) => void;
  llm?: LLMSelection;
  embedding?: EmbeddingSelection;
  vision?: LLMSelection;
  onLlmChange: (v: LLMSelection | undefined) => void;
  onEmbeddingChange: (v: EmbeddingSelection | undefined) => void;
  onVisionChange: (v: LLMSelection | undefined) => void;
  advancedOpen: boolean;
  onAdvancedOpenChange: (open: boolean) => void;
  /** When set, resolve LLM/Embed/Vision via tenant → server ladder (Create Workspace). */
  tenantId?: string | null;
  /**
   * Which eligible set the advanced pickers read.
   * First-run has no tenant, so the default is deployment until a tenant id exists.
   */
  choiceScope?: 'deployment' | 'tenant' | 'workspace';
  /** SPEC-179: verdicts on the pair this step will save. Absent on surfaces that do not judge yet. */
  eligibility?: WizardEligibility;
  /** Organization admins can open the offer from here. */
  canManageProviders?: boolean;
}

/**
 * SPEC-101 — Defaults-first model step (LAW-101-2 / LAW-101-3).
 * Happy path never mounts pickers. Advanced uses two-step provider → model.
 */
export function ModelDefaultsStep({
  draft,
  onChange,
  llm,
  embedding,
  vision,
  onLlmChange,
  onEmbeddingChange,
  onVisionChange,
  advancedOpen,
  onAdvancedOpenChange,
  tenantId,
  choiceScope,
  eligibility,
  canManageProviders = false,
}: ModelDefaultsStepProps) {
  const { t } = useTranslation();
  const useTenantLadder = Boolean(tenantId);
  const pickerScope = choiceScope ?? (tenantId ? 'tenant' : 'deployment');
  const inherited = useInheritedModelDefaults(tenantId ?? null);
  const llmModels = useLlmCapabilityModels();
  const reasoningSupported = useMemo(
    () =>
      supportedReasoningEffortsForModel(
        llmModels,
        llm?.provider,
        llm?.model,
      ),
    [llmModels, llm?.provider, llm?.model],
  );
  const thinkingSupported = useMemo(
    () => modelSupportsThinking(llmModels, llm?.provider, llm?.model),
    [llmModels, llm?.provider, llm?.model],
  );
  const reasoningEffectiveAuto = useMemo(
    () =>
      effectiveEffortWhenAuto(
        llmModels,
        llm?.provider,
        llm?.model,
        'fleet',
      ),
    [llmModels, llm?.provider, llm?.model],
  );

  const { isLoading, hasConfiguredDefaults } = inherited;
  const decisionStatus = useDecisionStatus(draft.decisionModel);
  const decisionModels = useDecisionModels();
  const decisionDraft: ExtractionModeDraft = {
    mode: 'inherit',
    enabled: draft.decisionEnabled,
    preset: draft.decisionPreset,
    model: draft.decisionModel,
    packSize: draft.decisionPackSize,
  };
  const decisionIssue = draftIssue(decisionDraft, decisionStatus.data?.limits);

  // Only force Advanced after defaults have finished loading and are incomplete.
  useEffect(() => {
    if (isLoading) return;
    if (!hasConfiguredDefaults && draft.useServerDefaults) {
      onChange({ useServerDefaults: false });
      onAdvancedOpenChange(true);
    }
  }, [
    isLoading,
    hasConfiguredDefaults,
    draft.useServerDefaults,
    onChange,
    onAdvancedOpenChange,
  ]);

  const showAdvanced = advancedOpen && !isLoading;
  /** One click from "not offered" to the picker, with the stale pick cleared. */
  const chooseAnother = (role: WizardRole) => {
    onChange({ useServerDefaults: false });
    onAdvancedOpenChange(true);
    if (role === 'llm') onLlmChange(undefined);
    if (role === 'embedding') onEmbeddingChange(undefined);
    if (role === 'vision') onVisionChange(undefined);
  };
  const tenantPrefix = t('onboarding.tenantDefaultShort', 'Organization default');
  const serverPrefix = t('onboarding.serverDefaultShort', 'Server default');
  const llmInheritLabel = formatInheritModelLabel({
    source: inherited.source,
    provider: inherited.defaultLlmProvider,
    model: inherited.defaultLlmModel,
    tenantPrefix,
    serverPrefix,
  });
  const embeddingInheritLabel = formatInheritModelLabel({
    source: inherited.source,
    provider: inherited.defaultEmbeddingProvider,
    model: inherited.defaultEmbeddingModel,
    tenantPrefix,
    serverPrefix,
  });
  const visionInheritLabel = formatInheritModelLabel({
    source: inherited.source,
    provider: inherited.defaultVisionProvider,
    model: inherited.defaultVisionModel,
    tenantPrefix,
    serverPrefix,
  });

  return (
    <div className="space-y-3" data-testid="wizard-step-models">
      <ServerDefaultsCard
        showCustomize={!showAdvanced}
        overridden={showAdvanced}
        defaults={{
          isLoading: inherited.isLoading,
          hasConfiguredDefaults: inherited.hasConfiguredDefaults,
          source: inherited.source,
          defaultLlmProvider: inherited.defaultLlmProvider,
          defaultLlmModel: inherited.defaultLlmModel,
          defaultEmbeddingProvider: inherited.defaultEmbeddingProvider,
          defaultEmbeddingModel: inherited.defaultEmbeddingModel,
          defaultVisionProvider: inherited.defaultVisionProvider,
          defaultVisionModel: inherited.defaultVisionModel,
          verdicts:
            eligibility && draft.useServerDefaults && !eligibility.nothingOffered
              ? eligibility.verdicts
              : undefined,
        }}
        onCustomize={() => {
          onChange({ useServerDefaults: false });
          onAdvancedOpenChange(true);
        }}
      />

      {eligibility ? (
        <WizardEligibilityNotice
          eligibility={eligibility}
          canManageProviders={canManageProviders}
          onChooseAnother={chooseAnother}
        />
      ) : null}

      <div data-testid="wizard-decision-engine">
        <DecisionEngineFields
          draft={decisionDraft}
          onChange={(patch) =>
            onChange({
              ...(patch.enabled !== undefined ? { decisionEnabled: patch.enabled } : {}),
              ...(patch.model !== undefined ? { decisionModel: patch.model } : {}),
              ...(patch.preset !== undefined ? { decisionPreset: patch.preset } : {}),
              ...(patch.packSize !== undefined ? { decisionPackSize: patch.packSize } : {}),
            })
          }
          limits={decisionStatus.data?.limits}
          issue={decisionIssue}
          forcedOn={decisionStatus.data?.activation === 'forced'}
          lockedByAdmin={decisionStatus.data?.activation === 'locked'}
          models={decisionModels.data?.models}
          modelsLoading={decisionModels.isLoading}
          host={
            decisionStatus.data?.provider?.base_url_host ??
            decisionStatus.data?.backend?.base_url_host ??
            null
          }
          status={decisionStatus.data}
          statusLoading={decisionStatus.isLoading}
          statusError={decisionStatus.isError}
          onRetryStatus={() => void decisionStatus.refetch()}
          statusRetrying={decisionStatus.isFetching}
        />
      </div>

      {showAdvanced ? (
        <div className="space-y-3" data-testid="wizard-models-advanced">
          <div className="flex items-center justify-between gap-2">
            <p className="text-sm font-medium">
              {t('onboarding.customModels', 'Custom model selection')}
            </p>
            {hasConfiguredDefaults ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="h-auto px-2 py-1 text-xs"
                onClick={() => {
                  onChange({ useServerDefaults: true });
                  onLlmChange(undefined);
                  onEmbeddingChange(undefined);
                  onVisionChange(undefined);
                  onAdvancedOpenChange(false);
                }}
                data-testid="wizard-models-use-defaults"
              >
                {useTenantLadder
                  ? t('onboarding.backToInheritedDefaults', 'Use organization defaults')
                  : t('onboarding.backToServerDefaults', 'Use server defaults')}
              </Button>
            ) : null}
          </div>
          <p className="text-xs text-muted-foreground">
            {t(
              'onboarding.customModelsHint',
              'Choose a provider, then a model.',
            )}
          </p>
          <div className="grid gap-3">
            <div className="grid gap-2">
              <Label>{t('workspace.llmModel', 'LLM Model')}</Label>
              <LLMModelSelector
                value={llm}
                onChange={onLlmChange}
                showCapabilityFilters={false}
                showUsageHint
                inheritLabel={llmInheritLabel}
                choiceScope={pickerScope}
              />
            </div>
            <div className="grid gap-2">
              <Label>{t('workspace.embeddingModel', 'Embedding Model')}</Label>
              <EmbeddingModelSelector
                value={embedding}
                onChange={onEmbeddingChange}
                inheritLabel={embeddingInheritLabel}
                choiceScope={pickerScope}
              />
            </div>
            <div className="grid gap-2">
              <Label>{t('workspace.visionLLM', 'Vision LLM')}</Label>
              <LLMModelSelector
                value={vision}
                onChange={onVisionChange}
                filterVision
                showCapabilityFilters={false}
                showUsageHint
                choiceScope={pickerScope}
                usage="vlm"
                inheritLabel={visionInheritLabel}
              />
            </div>
            <ReasoningEffortSelect
              value={draft.reasoningEffort}
              onChange={(reasoningEffort) => onChange({ reasoningEffort })}
              supported={reasoningSupported}
              thinkingSupported={thinkingSupported}
              effectiveWhenAuto={reasoningEffectiveAuto}
              label={t('onboarding.reasoningEffort', 'Default reasoning effort')}
              data-testid="wizard-reasoning-effort"
            />
          </div>
        </div>
      ) : null}

      {!isLoading && !hasConfiguredDefaults && !showAdvanced ? (
        <p
          className="text-sm text-amber-700 dark:text-amber-400"
          data-testid="wizard-models-required-hint"
        >
          {t(
            'onboarding.modelsRequiredHint',
            useTenantLadder
              ? 'Organization and server defaults are incomplete. Customize models to continue.'
              : 'Server defaults are incomplete. Customize models to continue.',
          )}
        </p>
      ) : null}
    </div>
  );
}
