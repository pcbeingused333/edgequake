'use client';

import { OptionCard } from '@/components/onboarding/option-card';
import { ServerDefaultsCard, type ResolvedDefaultsDisplay } from '@/components/onboarding/server-defaults-card';
import { WizardEligibilityNotice } from '@/components/onboarding/wizard-eligibility-notice';
import type { WizardEligibility } from '@/hooks/use-wizard-eligibility';
import type { WizardRole } from '@/lib/onboarding/defaults-eligibility';
import { useTranslation } from 'react-i18next';

export interface ModelSourceChoiceProps {
  inherited: ResolvedDefaultsDisplay;
  /** True when the workspace follows the inherited defaults. */
  followsDefaults: boolean;
  onUseDefaults: () => void;
  onCustomize: () => void;
  eligibility?: WizardEligibility;
  canManageProviders: boolean;
  /** Whose offer the refusal is about. The wizard judges this scope, not the fleet page. */
  offerScope?: 'deployment' | 'tenant' | 'workspace';
  onChooseAnother: (role: WizardRole) => void;
}

/**
 * The one decision on the models step: follow the defaults, or choose models.
 * Verdicts only decorate the defaults card while it is the active choice.
 */
export function ModelSourceChoice({
  inherited,
  followsDefaults,
  onUseDefaults,
  onCustomize,
  eligibility,
  canManageProviders,
  offerScope,
  onChooseAnother,
}: ModelSourceChoiceProps) {
  const { t } = useTranslation();
  const verdicts =
    eligibility && followsDefaults && !eligibility.nothingOffered ? eligibility.verdicts : undefined;

  return (
    <div className="space-y-3">
      <div
        role="radiogroup"
        aria-label={t('onboarding.modelSource', 'Where do the models come from?')}
        className="grid gap-3 md:grid-cols-5"
      >
        <ServerDefaultsCard
          className="md:col-span-3"
          selected={followsDefaults}
          onSelect={onUseDefaults}
          defaults={{ ...inherited, verdicts }}
        />
        <OptionCard
          className="md:col-span-2"
          selected={!followsDefaults}
          onSelect={onCustomize}
          title={t('onboarding.customizeModelsTitle', 'Choose models')}
          data-testid="server-defaults-customize"
        >
          <p className="text-xs text-muted-foreground">
            {t(
              'onboarding.customizeModelsHint',
              'Pick a provider and model for this workspace only.',
            )}
          </p>
        </OptionCard>
      </div>
      {eligibility ? (
        <WizardEligibilityNotice
          eligibility={eligibility}
          canManageProviders={canManageProviders}
          offerScope={offerScope}
          onChooseAnother={onChooseAnother}
        />
      ) : null}
    </div>
  );
}
