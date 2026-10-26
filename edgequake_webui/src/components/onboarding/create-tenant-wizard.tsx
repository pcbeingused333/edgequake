'use client';

import { WizardShell } from '@/components/onboarding/wizard-shell';
import { STEP_META, resolveStepLabels } from '@/components/onboarding/step-meta';
import { DocumentParsingStep } from '@/components/onboarding/steps/document-parsing-step';
import { ModelDefaultsStep } from '@/components/onboarding/steps/model-defaults-step';
import { ReviewStep } from '@/components/onboarding/steps/review-step';
import { TenantBasicsStep } from '@/components/onboarding/steps/tenant-basics-step';
import { WorkspaceBasicsStep } from '@/components/onboarding/steps/workspace-basics-step';
import { WorkspaceChunkingStep } from '@/components/onboarding/steps/workspace-chunking-step';
import { WorkspaceExtractBudgetStep } from '@/components/onboarding/steps/workspace-extract-budget-step';
import { WorkspaceExtractionStep } from '@/components/onboarding/steps/workspace-extraction-step';
import { ENTITY_PRESETS } from '@/constants/entity-presets';
import { EDGE_PRESETS, RELATION_PRESETS } from '@/constants/kg-schema-presets';
import { useServerModelDefaults } from '@/hooks/use-server-model-defaults';
import { useWizardDraftPicks } from '@/hooks/use-wizard-draft-picks';
import { setTenantContext } from '@/lib/api/client-context';
import {
  createTenant,
  createWorkspace,
  getWorkspaces,
  updateWorkspace,
} from '@/lib/api/edgequake';
import { buildTenantModelPayload, buildWorkspaceIngestPayload, buildWorkspaceModelPayload } from '@/lib/onboarding/model-payload';
import { useWizardDraftPersistence } from '@/lib/onboarding/use-wizard-draft-persistence';
import {
  EMPTY_WIZARD_DRAFT,
  wizardBlocker,
  clampStepIndex,
  draftForStorage,
  stepsForWizard,
  type WizardDraft,
  type WizardStepId,
} from '@/lib/onboarding/wizard-state';
import { describeBlocker, type BlockerTranslate } from '@/lib/onboarding/wizard-blocker-message';
import type { Tenant, Workspace } from '@/types';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export interface CreateTenantWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated?: (tenant: Tenant, workspace: Workspace) => void;
}

function initialTenantDraft(): WizardDraft {
  return {
    ...EMPTY_WIZARD_DRAFT,
    tenantName: '',
    workspaceName: 'Default Workspace',
    entityTypes: [...ENTITY_PRESETS.general.types],
    relationTypes: [...RELATION_PRESETS.general],
    relationEdges: EDGE_PRESETS.general.map((e) => ({ ...e })),
    kgSchemaPreset: 'general',
  };
}

/**
 * SPEC-101 — Guided Create Tenant (+ configure first workspace via PATCH Default).
 */
export function CreateTenantWizard({ open, onOpenChange, onCreated }: CreateTenantWizardProps) {
  const { t } = useTranslation();
  const steps = useMemo(() => stepsForWizard('create-tenant'), []);
  const baselineRef = useRef(initialTenantDraft());
  const [stepIndex, setStepIndex] = useState(0);
  const [draft, setDraft] = useState<WizardDraft>(initialTenantDraft);
  const [submitting, setSubmitting] = useState(false);
  const { hasConfiguredDefaults } = useServerModelDefaults();
  const {
    llm,
    embedding,
    vision,
    advancedOpen,
    commitLlm,
    commitEmbedding,
    commitVision,
    setAdvancedOpen,
  } = useWizardDraftPicks(draft, setDraft);
  const { clearDraft } = useWizardDraftPersistence(
    'create-tenant',
    open,
    draft,
    setDraft,
    stepIndex,
    setStepIndex,
    steps.length,
  );

  const stepId = steps[clampStepIndex(stepIndex, steps.length)];
  const meta = STEP_META[stepId];

  const patchDraft = useCallback((patch: Partial<WizardDraft>) => {
    setDraft((d) => ({ ...d, ...patch }));
  }, []);

  const advancedValid = Boolean(llm?.provider && embedding?.provider && vision?.provider);

  const blocker = wizardBlocker(stepId, draft, {
    hasConfiguredDefaults,
    advancedModelsValid: advancedValid,
  });
  const canGoNext = blocker === null;
  const blockedReason = describeBlocker(blocker, t as BlockerTranslate);
  const stepLabels = useMemo(() => resolveStepLabels(steps, t), [steps, t]);

  const isDirty =
    stepIndex > 0 ||
    JSON.stringify(draftForStorage(draft)) !==
      JSON.stringify(draftForStorage(baselineRef.current));

  const reset = useCallback(() => {
    const next = initialTenantDraft();
    baselineRef.current = next;
    setStepIndex(0);
    setDraft(next);
    clearDraft();
  }, [clearDraft]);

  const handleOpenChange = (next: boolean) => {
    if (!next) reset();
    onOpenChange(next);
  };

  const goToStep = (target: WizardStepId) => {
    const idx = steps.indexOf(target);
    if (idx >= 0) setStepIndex(idx);
  };

  const finalize = async () => {
    setSubmitting(true);
    try {
      const tenantModels = buildTenantModelPayload({
        useServerDefaults: draft.useServerDefaults,
        llm,
        embedding,
        vision,
        reasoningEffort: draft.reasoningEffort,
      });
      const tenant = await createTenant({
        name: draft.tenantName.trim(),
        description: draft.tenantDescription.trim() || undefined,
        ...tenantModels,
      });
      // Scope subsequent workspace calls to the new tenant (not the prior header context).
      setTenantContext(tenant.id);

      const list = await getWorkspaces(tenant.id);
      const autoWs =
        list.find((w) => w.slug === 'default') ??
        list.find((w) => w.name === 'Default Workspace') ??
        list[0];

      const wsModels = buildWorkspaceModelPayload({
        useServerDefaults: draft.useServerDefaults,
        llm,
        embedding,
        vision,
        reasoningEffort: draft.reasoningEffort,
      });

      const ingest = buildWorkspaceIngestPayload(draft, autoWs ? 'update' : 'create');

      let workspace: Workspace;
      if (autoWs) {
        workspace = await updateWorkspace(tenant.id, autoWs.id, {
          name: draft.workspaceName.trim(),
          description: draft.workspaceDescription.trim() || undefined,
          ...wsModels,
          ...ingest,
        });
      } else {
        workspace = await createWorkspace(tenant.id, {
          name: draft.workspaceName.trim(),
          slug: draft.workspaceSlug.trim() || undefined,
          description: draft.workspaceDescription.trim() || undefined,
          ...wsModels,
          ...ingest,
        });
      }

      // Success toast owned by caller (header / tenant-guard).
      onCreated?.(tenant, workspace);
      handleOpenChange(false);
    } catch (error) {
      toast.error(t('tenant.createFailed', 'Failed to create tenant'), {
        description: error instanceof Error ? error.message : 'Unknown error',
      });
    } finally {
      setSubmitting(false);
    }
  };

  const onNext = () => {
    if (stepIndex >= steps.length - 1) {
      void finalize();
      return;
    }
    setStepIndex((i) => clampStepIndex(i + 1, steps.length));
  };

  const body = (() => {
    switch (stepId) {
      case 'tenant-basics':
        return <TenantBasicsStep draft={draft} onChange={patchDraft} />;
      case 'models':
        return (
          <ModelDefaultsStep
            draft={draft}
            onChange={patchDraft}
            llm={llm}
            embedding={embedding}
            vision={vision}
            onLlmChange={commitLlm}
            onEmbeddingChange={commitEmbedding}
            onVisionChange={commitVision}
            advancedOpen={advancedOpen}
            onAdvancedOpenChange={setAdvancedOpen}
          />
        );
      case 'document-parsing':
        return <DocumentParsingStep draft={draft} onChange={patchDraft} />;
      case 'workspace-basics':
        return <WorkspaceBasicsStep draft={draft} onChange={patchDraft} />;
      case 'chunking':
        return <WorkspaceChunkingStep draft={draft} onChange={patchDraft} />;
      case 'extract-budget':
        return <WorkspaceExtractBudgetStep draft={draft} onChange={patchDraft} />;
      case 'extraction':
        return <WorkspaceExtractionStep draft={draft} onChange={patchDraft} />;
      case 'review':
        return (
          <ReviewStep
            draft={draft}
            includeAdmin={false}
            includeTenant
            llm={llm}
            embedding={embedding}
            vision={vision}
            onEditStep={goToStep}
            showDocumentParsing
          />
        );
      default:
        return null;
    }
  })();

  return (
    <WizardShell
      open={open}
      onOpenChange={handleOpenChange}
      title={t('tenant.createNew', 'Create New Tenant')}
      description={t(
        'tenant.createDescription',
        'Create a new tenant to organize your workspaces and data.',
      )}
      stepIndex={stepIndex}
      stepCount={steps.length}
      stepTitle={t(meta.titleKey, meta.title)}
      stepDescription={t(meta.descriptionKey, meta.description)}
      stepLabels={stepLabels}
      blockedReason={blockedReason}
      canGoNext={canGoNext}
      isLastStep={stepIndex >= steps.length - 1}
      isSubmitting={submitting}
      isDirty={isDirty}
      onBack={() => setStepIndex((i) => clampStepIndex(i - 1, steps.length))}
      onNext={onNext}
      testId="create-tenant-wizard"
    >
      {body}
    </WizardShell>
  );
}
