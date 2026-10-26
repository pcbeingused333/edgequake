'use client';

/**
 * One loading / error / empty row for every eligible-model list (ui-lists-status).
 * The error state names the problem and offers a retry; it never shows a silent blank.
 */

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export type ModelsStatusState = 'loading' | 'error' | 'empty';

export interface ModelsStatusProps {
  state: ModelsStatusState;
  /** Shown with the error state. Omit to hide the button. */
  onRetry?: () => void;
  /** Overrides the empty-state copy (for example a scope-specific sentence). */
  emptyText?: string;
  className?: string;
}

export function ModelsStatus({ state, onRetry, emptyText, className }: ModelsStatusProps) {
  const { t } = useTranslation();

  if (state === 'loading') {
    return (
      <div
        role="status"
        className={cn('flex items-center gap-2 rounded-lg bg-muted px-3 py-2', className)}
      >
        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" aria-hidden="true" />
        <span className="text-sm text-muted-foreground">
          {t('models.status.loading', 'Loading models…')}
        </span>
      </div>
    );
  }

  if (state === 'error') {
    return (
      <div
        role="alert"
        className={cn(
          'flex flex-wrap items-center justify-between gap-2 rounded-lg bg-muted px-3 py-2',
          className,
        )}
      >
        <span className="text-sm text-muted-foreground">
          {t('models.status.error', 'Could not load models.')}
        </span>
        {onRetry ? (
          <Button type="button" variant="outline" size="sm" onClick={onRetry}>
            {t('common.retry', 'Retry')}
          </Button>
        ) : null}
      </div>
    );
  }

  return (
    <div className={cn('rounded-lg bg-muted px-3 py-2', className)}>
      <span className="text-sm text-muted-foreground">
        {emptyText ?? t('models.status.empty', 'No eligible models yet.')}
      </span>
    </div>
  );
}
