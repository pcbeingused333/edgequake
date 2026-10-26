'use client';

/**
 * SPEC-179 — what the models step says when a default or pick is not offered any more,
 * when a size is missing, or when nothing is offered at all. Each message has one fix.
 */

import Link from 'next/link';
import { useTranslation } from 'react-i18next';
import { TriangleAlert } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { WizardEligibility } from '@/hooks/use-wizard-eligibility';
import {
  blockedRoles,
  ROLE_LABEL,
  type ModelPair,
  type RoleStatus,
  type WizardRole,
} from '@/lib/onboarding/defaults-eligibility';
import { sectionHref } from '@/lib/settings/registry';

const pairLabel = (pair: ModelPair) =>
  pair.provider && pair.model ? `${pair.provider}/${pair.model}` : '';

function reasonText(
  t: ReturnType<typeof useTranslation>['t'],
  status: RoleStatus,
  pair: ModelPair,
): string {
  if (status === 'dimension_missing') {
    return t(
      'onboarding.eligibility.dimensionMissing',
      '{{model}} has no registered vector size. Pick another embedding model, or ask a deployment admin to set it.',
      { model: pairLabel(pair) },
    );
  }
  return t(
    'onboarding.eligibility.notOffered',
    '{{model}} is not offered to this organization any more.',
    { model: pairLabel(pair) },
  );
}

export function WizardEligibilityNotice({
  eligibility,
  canManageProviders,
  onChooseAnother,
}: {
  eligibility: WizardEligibility;
  /** Organization admins get a link to the offer; everyone else is told whom to ask. */
  canManageProviders: boolean;
  /** Opens the custom pickers so the user can replace the blocked role. */
  onChooseAnother: (role: WizardRole) => void;
}) {
  const { t } = useTranslation();
  if (eligibility.isLoading) return null;

  if (eligibility.isError) {
    return (
      <p role="alert" className="text-sm text-destructive" data-testid="wizard-eligibility-error">
        {t('onboarding.eligibility.loadError', 'Could not check which models are offered. Try again in a moment.')}
      </p>
    );
  }

  if (eligibility.nothingOffered) {
    return (
      <div
        role="alert"
        className="space-y-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm"
        data-testid="wizard-nothing-offered"
      >
        <p className="font-medium">
          {t('onboarding.eligibility.nothingTitle', 'No provider is offered to this organization yet')}
        </p>
        <p className="text-muted-foreground">
          {canManageProviders
            ? t('onboarding.eligibility.nothingAdmin', 'Turn on at least one provider, then come back to create the workspace.')
            : t('onboarding.eligibility.nothingMember', 'Ask an organization admin to turn on a provider.')}
        </p>
        {canManageProviders ? (
          <Button asChild size="sm" variant="outline">
            <Link href={sectionHref('org-providers')} data-testid="wizard-open-providers">
              {t('onboarding.eligibility.openProviders', 'Open organization providers')}
            </Link>
          </Button>
        ) : null}
      </div>
    );
  }

  const roles = blockedRoles(eligibility.verdicts);
  if (roles.length === 0) return null;
  return (
    <ul
      role="alert"
      className="space-y-2 rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm"
      data-testid="wizard-eligibility-blocked"
    >
      {roles.map((role) => (
        <li key={role} className="flex flex-wrap items-center justify-between gap-2" data-testid={`wizard-blocked-${role}`}>
          <span className="flex min-w-0 items-start gap-2">
            <TriangleAlert className="mt-0.5 h-4 w-4 shrink-0 text-destructive" aria-hidden />
            <span>
              <span className="font-medium">{ROLE_LABEL[role]}. </span>
              {reasonText(t, eligibility.verdicts[role], eligibility.pairs[role])}
            </span>
          </span>
          <Button type="button" size="sm" variant="outline" onClick={() => onChooseAnother(role)}>
            {t('onboarding.eligibility.chooseAnother', 'Choose another')}
          </Button>
        </li>
      ))}
    </ul>
  );
}
