'use client';

/**
 * Segmented, labelled wizard stepper (list of steps, `aria-current` on the active one).
 * Finished steps are buttons that jump back; upcoming steps are inert.
 */

import { cn } from '@/lib/utils';
import { Check } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface WizardStepperProps {
  labels: readonly string[];
  currentIndex: number;
  /** Jump to an earlier step. Omit to make the stepper read-only. */
  onSelect?: (index: number) => void;
  disabled?: boolean;
}

type StepState = 'done' | 'current' | 'todo';

const stateOf = (index: number, current: number): StepState =>
  index < current ? 'done' : index === current ? 'current' : 'todo';

const BAR: Record<StepState, string> = {
  done: 'bg-primary',
  current: 'bg-primary ring-2 ring-primary/25',
  todo: 'bg-border',
};

function StepSegment({
  label,
  index,
  state,
  onSelect,
  jumpLabel,
}: {
  label: string;
  index: number;
  state: StepState;
  onSelect?: (index: number) => void;
  jumpLabel: string;
}) {
  const clickable = state === 'done' && Boolean(onSelect);
  const content = (
    <>
      <span aria-hidden className={cn('block h-1 w-full rounded-full transition-colors', BAR[state])} />
      <span
        className={cn(
          'mt-1.5 flex items-center gap-1 truncate text-[11px] leading-none',
          state === 'current' ? 'font-semibold text-foreground' : 'text-muted-foreground',
          // On narrow screens only the active label fits.
          state !== 'current' && 'max-sm:hidden',
        )}
      >
        {state === 'done' ? <Check aria-hidden className="h-3 w-3 shrink-0 text-primary" /> : null}
        <span className="truncate">{label}</span>
      </span>
    </>
  );

  return (
    <li
      className="min-w-0 flex-1"
      aria-current={state === 'current' ? 'step' : undefined}
      data-testid={`wizard-stepper-item-${index}`}
      data-state={state}
    >
      {clickable ? (
        <button
          type="button"
          className="block w-full min-w-0 rounded-sm text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => onSelect?.(index)}
          aria-label={jumpLabel}
          data-testid={`wizard-stepper-jump-${index}`}
        >
          {content}
        </button>
      ) : (
        <div className="min-w-0">{content}</div>
      )}
    </li>
  );
}

export function WizardStepper({ labels, currentIndex, onSelect, disabled }: WizardStepperProps) {
  const { t } = useTranslation();
  return (
    <nav aria-label={t('onboarding.stepsNav', 'Wizard steps')} data-testid="wizard-progress">
      <ol className="flex gap-1.5">
        {labels.map((label, index) => (
          <StepSegment
            key={`${index}-${label}`}
            label={label}
            index={index}
            state={stateOf(index, currentIndex)}
            onSelect={disabled ? undefined : onSelect}
            jumpLabel={t('onboarding.backToStep', 'Back to {{step}}', { step: label })}
          />
        ))}
      </ol>
    </nav>
  );
}
