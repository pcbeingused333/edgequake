'use client';

/**
 * Selectable option card (a single-choice radio rendered as a card).
 * One accent border + soft ring when selected; fully keyboard and screen-reader operable.
 */

import { cn } from '@/lib/utils';
import { Check } from 'lucide-react';

export interface OptionCardProps
  extends Omit<React.ComponentPropsWithoutRef<'div'>, 'onSelect' | 'title'> {
  selected: boolean;
  onSelect: () => void;
  title: React.ReactNode;
  /** Small pill next to the title, e.g. “Recommended”. */
  badge?: React.ReactNode;
  disabled?: boolean;
}

export function OptionCard({
  selected,
  onSelect,
  title,
  badge,
  disabled = false,
  className,
  children,
  onKeyDown,
  ...rest
}: OptionCardProps) {
  return (
    <div
      role="radio"
      aria-checked={selected}
      aria-disabled={disabled || undefined}
      tabIndex={disabled ? -1 : 0}
      data-selected={selected ? 'true' : 'false'}
      onClick={disabled ? undefined : onSelect}
      onKeyDown={(e) => {
        onKeyDown?.(e);
        if (e.defaultPrevented || disabled) return;
        if (e.key === ' ' || e.key === 'Enter') {
          e.preventDefault();
          onSelect();
        }
      }}
      className={cn(
        'relative flex min-w-0 flex-col gap-2 rounded-xl border bg-card p-3.5 text-left outline-none transition-[border-color,box-shadow,background-color]',
        'focus-visible:ring-2 focus-visible:ring-ring',
        disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer hover:border-primary/50',
        selected && 'border-primary bg-primary/[0.04] ring-[3px] ring-primary/15',
        className,
      )}
      {...rest}
    >
      <div className="flex items-center gap-2">
        <span
          aria-hidden
          className={cn(
            'flex h-4 w-4 shrink-0 items-center justify-center rounded-full border',
            selected ? 'border-primary bg-primary text-primary-foreground' : 'border-input',
          )}
        >
          {selected ? <Check className="h-3 w-3" strokeWidth={3} /> : null}
        </span>
        <span className="min-w-0 flex-1 truncate text-sm font-semibold">{title}</span>
        {badge ? (
          <span className="shrink-0 rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary">
            {badge}
          </span>
        ) : null}
      </div>
      {children}
    </div>
  );
}
