"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useState, type FormEvent, type ReactNode } from "react";

/** Building blocks shared by every Access popover. Each one does one job. */

/** Popover shell: a title, a body, and an optional footer. Controlled or not. */
export function LabelPopover({
  trigger,
  title,
  open,
  onOpenChange,
  align = "end",
  testId,
  footer,
  children,
}: {
  trigger: ReactNode;
  title: string;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  align?: "start" | "center" | "end";
  testId?: string;
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Popover open={open} onOpenChange={onOpenChange}>
      <PopoverTrigger asChild>{trigger}</PopoverTrigger>
      <PopoverContent align={align} className="w-80 p-0" data-testid={testId}>
        <h3 className="border-b px-4 py-3 text-sm font-medium">{title}</h3>
        <div className="space-y-4 px-4 py-3">{children}</div>
        {footer ? <div className="border-t px-4 py-2.5">{footer}</div> : null}
      </PopoverContent>
    </Popover>
  );
}

export interface ChecklistOption {
  id: string;
  label: string;
  /** Optional muted line under the label. */
  hint?: string;
}

/** A labelled list of checkboxes. The whole row is the click target. */
export function ChecklistGroup({
  label,
  options,
  selected,
  onToggle,
  emptyText,
  testId,
}: {
  label: string;
  options: ChecklistOption[];
  selected: string[];
  onToggle: (id: string) => void;
  emptyText: string;
  testId?: string;
}) {
  return (
    <fieldset className="space-y-1.5" data-testid={testId}>
      <legend className="mb-1 text-xs font-medium text-muted-foreground">{label}</legend>
      {options.length === 0 ? (
        <p className="px-2 text-sm text-muted-foreground">{emptyText}</p>
      ) : (
        <ul className="max-h-56 space-y-0.5 overflow-y-auto -mx-2">
          {options.map((option) => (
            <li key={option.id}>
              <label className="flex cursor-pointer items-center gap-2.5 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                <Checkbox
                  checked={selected.includes(option.id)}
                  onCheckedChange={() => onToggle(option.id)}
                  aria-label={option.label}
                />
                <span className="min-w-0 flex-1">
                  <span className="block truncate">{option.label}</span>
                  {option.hint ? (
                    <span className="block truncate text-xs text-muted-foreground">{option.hint}</span>
                  ) : null}
                </span>
              </label>
            </li>
          ))}
        </ul>
      )}
    </fieldset>
  );
}

export interface SelectChoice<T extends string> {
  value: T;
  label: string;
}

/** A labelled single choice, full width inside a popover. */
export function ChoiceField<T extends string>({
  label,
  value,
  choices,
  onChange,
}: {
  label: string;
  value: T;
  choices: readonly SelectChoice<T>[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="space-y-1.5">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      <Select value={value} onValueChange={(next) => onChange(next as T)}>
        <SelectTrigger className="h-9 w-full" aria-label={label}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {choices.map((choice) => (
            <SelectItem key={choice.value} value={choice.value}>
              {choice.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}

/** One text field and a submit button. Clears only after the save resolves. */
export function NameForm({
  placeholder,
  actionLabel,
  onSubmit,
}: {
  placeholder: string;
  actionLabel: string;
  onSubmit: (value: string) => Promise<unknown>;
}) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const trimmed = value.trim();

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!trimmed || busy) return;
    setBusy(true);
    try {
      await onSubmit(trimmed);
      setValue("");
    } catch {
      // The caller reports the error; keep the text so the user can fix it.
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="flex items-center gap-2" onSubmit={handleSubmit}>
      <Input
        className="h-9 min-w-0 flex-1"
        placeholder={placeholder}
        aria-label={placeholder}
        value={value}
        onChange={(event) => setValue(event.target.value)}
      />
      <Button type="submit" size="sm" variant="outline" disabled={!trimmed || busy}>
        {actionLabel}
      </Button>
    </form>
  );
}
