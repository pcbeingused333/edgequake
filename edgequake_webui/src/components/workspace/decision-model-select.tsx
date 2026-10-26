'use client';

/**
 * Decision model combobox: list from the local Ollama host, capable models first.
 * Typing a tag that is not in the list still selects it (not yet pulled).
 */

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import {
  DECISION_MODEL_PLACEHOLDER,
  type DecisionListedModel,
} from '@/constants/extraction-mode';
import { cn } from '@/lib/utils';
import { Check, ChevronDown, Loader2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface DecisionModelSelectProps {
  value: string;
  onChange: (model: string) => void;
  models: DecisionListedModel[];
  loading?: boolean;
  disabled?: boolean;
  invalid?: boolean;
}

function sortModels(models: DecisionListedModel[], current: string): DecisionListedModel[] {
  return [...models].sort((a, b) => {
    if (a.name === current && b.name !== current) return -1;
    if (b.name === current && a.name !== current) return 1;
    const ac = a.decision_capable === true ? 0 : 1;
    const bc = b.decision_capable === true ? 0 : 1;
    if (ac !== bc) return ac - bc;
    return a.name.localeCompare(b.name);
  });
}

export function DecisionModelSelect({
  value,
  onChange,
  models,
  loading = false,
  disabled = false,
  invalid = false,
}: DecisionModelSelectProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');
  const current = value.trim() || DECISION_MODEL_PLACEHOLDER;
  const sorted = useMemo(() => {
    const list = sortModels(models, current);
    if (current && !list.some((m) => m.name === current)) {
      return [{ name: current, decision_capable: null }, ...list];
    }
    return list;
  }, [models, current]);

  // A name the host did not list is kept as typed, but it is marked as unverified.
  const listed = useMemo(() => new Set(models.map((m) => m.name)), [models]);
  const verifiedBadge = (name: string) =>
    listed.has(name) ? null : (
      <Badge variant="outline" className="px-1.5 py-0 text-[10px]" data-testid="decision-model-unverified">
        {t('extractionMode.engine.notVerified', 'Not verified on host')}
      </Badge>
    );

  const needle = search.trim().toLowerCase();
  const filtered = needle
    ? sorted.filter((m) => m.name.toLowerCase().includes(needle))
    : sorted;
  const custom =
    needle && !sorted.some((m) => m.name.toLowerCase() === needle) && !/\s/.test(search.trim())
      ? search.trim()
      : null;

  const pick = (name: string) => {
    onChange(name);
    setOpen(false);
    setSearch('');
  };

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setSearch('');
      }}
    >
      <PopoverTrigger asChild>
        <Button
          type="button"
          variant="outline"
          role="combobox"
          aria-expanded={open}
          aria-invalid={invalid}
          disabled={disabled}
          className={cn(
            'h-10 w-full min-w-0 justify-between gap-2 overflow-hidden bg-background font-normal',
            invalid && 'border-destructive',
          )}
          data-testid="decision-model"
        >
          <span className="min-w-0 flex-1 truncate text-left font-mono text-sm">{current}</span>
          {loading && models.length === 0 ? (
            <Loader2 className="h-4 w-4 shrink-0 animate-spin opacity-50" />
          ) : (
            <ChevronDown className="h-4 w-4 shrink-0 opacity-50" />
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        className="z-[400] w-(--radix-popover-trigger-width) min-w-[min(100%,18rem)] p-0"
        align="start"
        side="bottom"
        avoidCollisions={false}
        onOpenAutoFocus={(event) => event.preventDefault()}
      >
        <Command shouldFilter={false} loop>
          <CommandInput
            placeholder={t('extractionMode.engine.searchModels', 'Search models…')}
            value={search}
            onValueChange={setSearch}
            data-testid="decision-model-search"
          />
          <CommandList className="max-h-56 overflow-y-auto" data-testid="decision-model-list">
            <CommandEmpty>{t('extractionMode.engine.noModels', 'No models on this host.')}</CommandEmpty>
            {custom ? (
              <CommandGroup>
                <CommandItem
                  value={`custom-${custom.replace(/[^a-zA-Z0-9]+/g, '-')}`}
                  onSelect={() => pick(custom)}
                  data-testid={`decision-model-option-${custom}`}
                >
                  <span className="min-w-0 flex-1 truncate">
                    {t('extractionMode.engine.useTag', 'Use {{tag}}', { tag: custom })}
                  </span>
                  {verifiedBadge(custom)}
                </CommandItem>
              </CommandGroup>
            ) : null}
            <CommandGroup heading={t('extractionMode.engine.capableHeading', 'Decision-capable')}>
              {filtered
                .filter((m) => m.decision_capable !== false)
                .map((m, i) => (
                  <CommandItem
                    key={m.name}
                    value={`cap-${i}-${m.name.replace(/[^a-zA-Z0-9]+/g, '-')}`}
                    onSelect={() => pick(m.name)}
                    data-testid={`decision-model-option-${m.name}`}
                  >
                    <Check
                      className={cn(
                        'h-4 w-4 shrink-0',
                        current === m.name ? 'opacity-100' : 'opacity-0',
                      )}
                    />
                    <span className="min-w-0 flex-1 truncate font-mono text-sm">{m.name}</span>
                    {verifiedBadge(m.name)}
                  </CommandItem>
                ))}
            </CommandGroup>
            {filtered.some((m) => m.decision_capable === false) ? (
              <CommandGroup heading={t('extractionMode.engine.otherHeading', 'Other models')}>
                {filtered
                  .filter((m) => m.decision_capable === false)
                  .map((m, i) => (
                    <CommandItem
                      key={m.name}
                      value={`other-${i}-${m.name.replace(/[^a-zA-Z0-9]+/g, '-')}`}
                      onSelect={() => pick(m.name)}
                      data-testid={`decision-model-option-${m.name}`}
                    >
                      <Check
                        className={cn(
                          'h-4 w-4 shrink-0',
                          current === m.name ? 'opacity-100' : 'opacity-0',
                        )}
                      />
                      <span className="min-w-0 flex-1 truncate font-mono text-sm">{m.name}</span>
                      <Badge variant="outline" className="px-1.5 py-0 text-[10px]">
                        {t('extractionMode.engine.notCapable', 'not decision-capable')}
                      </Badge>
                    </CommandItem>
                  ))}
              </CommandGroup>
            ) : null}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
}
