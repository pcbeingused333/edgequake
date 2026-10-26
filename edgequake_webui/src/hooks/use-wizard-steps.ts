'use client';

import { clampStepIndex, type WizardStepId } from '@/lib/onboarding/wizard-state';
import { useCallback, useState } from 'react';

/**
 * Step cursor shared by every wizard: index, current id, back / next / jump.
 * Persistence hydrates through `setStepIndex`, so it is returned as-is.
 */
export function useWizardSteps(steps: readonly WizardStepId[]) {
  const [stepIndex, setStepIndex] = useState(0);
  const last = steps.length - 1;
  const stepId = steps[clampStepIndex(stepIndex, steps.length)];

  const goBack = useCallback(
    () => setStepIndex((i) => clampStepIndex(i - 1, steps.length)),
    [steps.length],
  );
  const advance = useCallback(
    () => setStepIndex((i) => clampStepIndex(i + 1, steps.length)),
    [steps.length],
  );
  const goToIndex = useCallback(
    (index: number) => setStepIndex(clampStepIndex(index, steps.length)),
    [steps.length],
  );
  const goToStep = useCallback(
    (target: WizardStepId) => {
      const idx = steps.indexOf(target);
      if (idx >= 0) setStepIndex(idx);
    },
    [steps],
  );
  const restart = useCallback(() => setStepIndex(0), []);

  return {
    stepIndex,
    setStepIndex,
    stepId,
    isLastStep: stepIndex >= last,
    goBack,
    advance,
    goToIndex,
    goToStep,
    restart,
  };
}
