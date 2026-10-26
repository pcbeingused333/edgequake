'use client';

import { useWizardEligibility } from '@/hooks/use-wizard-eligibility';
import type { WizardEligibilityInput } from '@/hooks/use-wizard-eligibility';
import { hasAnyModelPick } from '@/lib/onboarding/wizard-picks';
import {
  describeBlocker,
  type BlockerTranslate,
} from '@/lib/onboarding/wizard-blocker-message';
import {
  wizardBlocker,
  type WizardDraft,
  type WizardStepId,
} from '@/lib/onboarding/wizard-state';
import { useTranslation } from 'react-i18next';

export interface WizardGateInput extends Omit<WizardEligibilityInput, 'useDefaults'> {
  stepId: WizardStepId;
  draft: WizardDraft;
  hasConfiguredDefaults: boolean;
  /** Reconfigure: Apply needs at least one change. */
  hasConfigChanges?: boolean;
}

/**
 * One answer to “may the user continue, and if not, why?” for a wizard step.
 * Create and Reconfigure share it, so the rule and the sentence cannot drift.
 */
export function useWizardGate(input: WizardGateInput) {
  const { t } = useTranslation();
  const { stepId, draft, hasConfiguredDefaults, hasConfigChanges } = input;
  const eligibility = useWizardEligibility({ ...input, useDefaults: draft.useServerDefaults });
  const anyPick = hasAnyModelPick({
    llm: input.llm,
    embedding: input.embedding,
    vision: input.vision,
  });
  const eligibilityOk = !eligibility.blocked && !eligibility.nothingOffered;
  const code = wizardBlocker(stepId, draft, {
    hasConfiguredDefaults,
    advancedModelsValid: anyPick,
    eligibilityOk,
    hasConfigChanges,
  });
  const blockedReason = describeBlocker(code, t as BlockerTranslate, {
    verdicts: eligibility.verdicts,
    nothingOffered: eligibility.nothingOffered,
    scope: input.scope,
  });
  return { eligibility, eligibilityOk, canGoNext: code === null, blockedReason };
}
