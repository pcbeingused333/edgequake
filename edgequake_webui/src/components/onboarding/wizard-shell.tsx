'use client';

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { WizardStepper } from '@/components/onboarding/wizard-stepper';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { isPortaledChoiceMenu } from '@/lib/onboarding/portaled-menu';
import { cn } from '@/lib/utils';
import { Info, Loader2 } from 'lucide-react';
import { useEffect, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface WizardShellProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description?: string;
  stepIndex: number;
  stepCount: number;
  stepTitle: string;
  stepDescription?: string;
  children: React.ReactNode;
  canGoNext: boolean;
  isLastStep: boolean;
  isSubmitting?: boolean;
  onBack: () => void;
  onNext: () => void;
  onCancel?: () => void;
  nextLabel?: string;
  finishLabel?: string;
  className?: string;
  testId?: string;
  /** When false, ignore outside/Escape dismiss (first-run). Default true. */
  dismissible?: boolean;
  /** Hide Cancel button (first-run). Default false. */
  hideCancel?: boolean;
  /** Forwarded to DialogContent. Default true. */
  showCloseButton?: boolean;
  /** When true, Cancel/close asks for confirmation. */
  isDirty?: boolean;
  /** Short per-step labels for the stepper (one per step). */
  stepLabels?: readonly string[];
  /** Jump back to a finished step from the stepper. */
  onStepSelect?: (index: number) => void;
  /** Why Next is disabled; shown beside the buttons so it never looks broken. */
  blockedReason?: string | null;
}

/**
 * SPEC-101 — Shared multi-step dialog chrome (LAW-101-1, LAW-101-8/10).
 */
export function WizardShell({
  open,
  onOpenChange,
  title,
  description,
  stepIndex,
  stepCount,
  stepTitle,
  stepDescription,
  children,
  canGoNext,
  isLastStep,
  isSubmitting = false,
  onBack,
  onNext,
  onCancel,
  nextLabel,
  finishLabel,
  className,
  testId = 'wizard-shell',
  dismissible = true,
  hideCancel = false,
  showCloseButton = true,
  isDirty = false,
  stepLabels,
  onStepSelect,
  blockedReason = null,
}: WizardShellProps) {
  const { t } = useTranslation();
  const stepTitleId = useId();
  const stepHeadingRef = useRef<HTMLHeadingElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const labels = stepLabels ?? Array.from({ length: stepCount }, (_, i) => String(i + 1));
  const stepOfLabel = t('onboarding.stepOf', 'Step {{current}} of {{total}}', {
    current: stepIndex + 1,
    total: stepCount,
  });
  const valueText = `${stepOfLabel}: ${stepTitle}`;

  /** Step bodies mark their primary field `data-autofocus`; otherwise the step heading takes focus. */
  const focusStep = () => {
    const field = bodyRef.current?.querySelector<HTMLElement>('[data-autofocus]');
    (field ?? stepHeadingRef.current)?.focus();
  };

  useEffect(() => {
    if (open) focusStep();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- refs only; re-run per step
  }, [open, stepIndex]);

  const requestClose = () => {
    if (!dismissible) return;
    if (isDirty && !isSubmitting) {
      setConfirmDiscard(true);
      return;
    }
    onCancel?.();
    onOpenChange(false);
  };

  const confirmClose = () => {
    setConfirmDiscard(false);
    onCancel?.();
    onOpenChange(false);
  };

  return (
    <>
      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (!next) {
            requestClose();
            return;
          }
          onOpenChange(next);
        }}
      >
        <DialogContent
          className={cn(
            // SPEC-114 layout: DialogContent defaults to `grid` — force a column flex
            // shell so sticky chrome + scroll body + footer share max height correctly.
            '!flex !flex-col gap-0 overflow-hidden p-0',
            'h-[min(88vh,52rem)] w-[calc(100%-1.5rem)] sm:!max-w-3xl lg:!max-w-4xl',
            className,
          )}
          data-testid={testId}
          showCloseButton={showCloseButton && dismissible}
          onOpenAutoFocus={(e) => {
            e.preventDefault();
            focusStep();
          }}
          onPointerDownOutside={(e) => {
            // The model menus portal to body. That click is outside the dialog,
            // but it is the selection — dismissing or opening discard would eat it.
            if (isPortaledChoiceMenu(e.detail.originalEvent.target)) {
              e.preventDefault();
              return;
            }
            if (!dismissible || isDirty) e.preventDefault();
            if (isDirty && dismissible) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
          onEscapeKeyDown={(e) => {
            if (!dismissible) {
              e.preventDefault();
              return;
            }
            if (isDirty) {
              e.preventDefault();
              setConfirmDiscard(true);
            }
          }}
        >
          {/* Sticky chrome — stays visible while the step body scrolls */}
          <div className="shrink-0 space-y-3 border-b px-5 pt-5 pb-3 sm:px-6">
            <DialogHeader className="gap-0.5 pr-8 text-left">
              {/* Do not pass a custom `id` — Radix TitleWarning looks up context.titleId. */}
              <DialogTitle className="text-base sm:text-lg">{title}</DialogTitle>
              <DialogDescription className={cn(!description && 'sr-only', 'max-sm:sr-only')}>
                {description ??
                  t('onboarding.wizardDialogHint', 'Multi-step creation wizard')}
              </DialogDescription>
            </DialogHeader>

            <WizardStepper
              labels={labels}
              currentIndex={stepIndex}
              onSelect={onStepSelect}
              disabled={isSubmitting}
            />
            <div className="sr-only" aria-live="polite" aria-atomic="true" data-testid="wizard-step-live">
              {valueText}
            </div>
            {/* Focus target for step changes. */}
            <h3
              ref={stepHeadingRef}
              id={stepTitleId}
              tabIndex={-1}
              className="sr-only"
              data-testid="wizard-step-title"
            >
              {stepTitle}
            </h3>
            <div className="flex items-baseline justify-between gap-3">
              <span className="text-sm font-semibold" data-testid="wizard-step-label">
                {stepTitle}
              </span>
              <span className="shrink-0 text-xs text-muted-foreground" data-testid="wizard-step-counter">
                {stepOfLabel}
              </span>
            </div>
          </div>

          <div
            ref={bodyRef}
            className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-5 py-4 sm:px-6"
            data-testid="wizard-step-scroll"
            onKeyDown={(e) => {
              // Opt-in: only fields marked `data-wizard-enter` advance on Enter.
              const target = e.target as HTMLElement;
              if (e.key !== 'Enter' || e.shiftKey || !target.hasAttribute('data-wizard-enter')) return;
              e.preventDefault();
              if (canGoNext && !isSubmitting && !isLastStep) onNext();
            }}
          >
            <div data-testid="wizard-step-body" role="group" aria-labelledby={stepTitleId} className="space-y-4">
              {stepDescription ? (
                <p className="text-sm text-muted-foreground" data-testid="wizard-step-description">
                  {stepDescription}
                </p>
              ) : null}
              {children}
            </div>
          </div>

          <div className="relative z-10 flex shrink-0 flex-col gap-2 border-t bg-background px-5 py-3 sm:flex-row sm:items-center sm:gap-3 sm:px-6">
            <div
              role="status"
              aria-live="polite"
              className="flex-1 text-xs text-muted-foreground empty:hidden"
              data-testid="wizard-blocked-reason"
            >
              {!canGoNext && blockedReason ? (
                <span className="flex items-start gap-1.5">
                  <Info aria-hidden className="mt-px h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
                  <span>{blockedReason}</span>
                </span>
              ) : null}
            </div>
            <div className="flex gap-2 max-sm:w-full">
              {!hideCancel ? (
                <Button
                  type="button"
                  variant="ghost"
                  onClick={requestClose}
                  disabled={isSubmitting}
                  data-testid="wizard-cancel"
                  className="max-sm:flex-1"
                >
                  {t('common.cancel', 'Cancel')}
                </Button>
              ) : null}
              <Button
                type="button"
                variant="outline"
                onClick={onBack}
                disabled={stepIndex === 0 || isSubmitting}
                data-testid="wizard-back"
                className="max-sm:flex-1"
              >
                {t('common.back', 'Back')}
              </Button>
              <Button
                type="button"
                onClick={onNext}
                disabled={!canGoNext || isSubmitting}
                data-testid={isLastStep ? 'wizard-finish' : 'wizard-next'}
                className="max-sm:flex-1"
              >
                {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                {isLastStep
                  ? finishLabel ?? t('common.create', 'Create')
                  : nextLabel ?? t('common.next', 'Next')}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmDiscard} onOpenChange={setConfirmDiscard}>
        <AlertDialogContent data-testid="wizard-discard-confirm">
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t('onboarding.discardTitle', 'Discard changes?')}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t(
                'onboarding.discardDescription',
                'You have unsaved wizard progress. Closing will discard it.',
              )}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel data-testid="wizard-discard-keep">
              {t('onboarding.keepEditing', 'Keep editing')}
            </AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmClose}
              data-testid="wizard-discard-confirm-action"
            >
              {t('onboarding.discardConfirm', 'Discard')}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
