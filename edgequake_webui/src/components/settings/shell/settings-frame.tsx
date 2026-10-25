"use client";

/**
 * SPEC-175 — the page primitives every settings section is built from.
 *
 * - SettingsFrame: one h1 per section, focused on navigation (screen-reader announcement).
 * - SettingsGroup: a titled block separated by a rule (gr8r `.sblock`).
 * - SettingRow: label, description and one control (gr8r `.srow`). The control receives the
 *   label id so every control is named by its visible label (WCAG 2.5.3, 4.1.2).
 */

import { cn } from "@/lib/utils";
import { useEffect, useId, useRef, type ReactNode } from "react";

export function SettingsFrame({
  title,
  lead,
  children,
  actions,
}: {
  title: string;
  lead?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  // Move focus to the new section heading so keyboard and screen-reader users land on
  // the content that changed. `preventScroll` keeps the nav position stable.
  useEffect(() => {
    headingRef.current?.focus({ preventScroll: true });
  }, []);

  return (
    <div className="space-y-6" data-testid="settings-frame">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1
            ref={headingRef}
            tabIndex={-1}
            className="text-xl font-semibold tracking-tight text-foreground outline-none"
          >
            {title}
          </h1>
          {lead ? <p className="mt-1 max-w-prose text-sm text-muted-foreground">{lead}</p> : null}
        </div>
        {actions ? <div className="flex shrink-0 items-center gap-2">{actions}</div> : null}
      </header>
      {children}
    </div>
  );
}

export function SettingsGroup({
  title,
  description,
  children,
  className,
  id,
}: {
  title?: string;
  description?: string;
  children: ReactNode;
  className?: string;
  id?: string;
}) {
  const headingId = useId();
  return (
    <section
      id={id}
      aria-labelledby={title ? headingId : undefined}
      className={cn("space-y-1", className)}
    >
      {title ? (
        <div className="border-b pb-2">
          <h2 id={headingId} className="text-sm font-semibold text-foreground">
            {title}
          </h2>
          {description ? <p className="mt-0.5 text-sm text-muted-foreground">{description}</p> : null}
        </div>
      ) : null}
      <div className="divide-y">{children}</div>
    </section>
  );
}

export function SettingRow({
  label,
  description,
  control,
  className,
  testId,
}: {
  label: string;
  description?: ReactNode;
  /** Receives the id of the label element, for `aria-labelledby`. */
  control: (labelId: string) => ReactNode;
  className?: string;
  testId?: string;
}) {
  const labelId = useId();
  return (
    <div
      role="group"
      aria-labelledby={labelId}
      data-testid={testId}
      className={cn(
        "grid grid-cols-1 items-center gap-3 py-4 sm:grid-cols-[minmax(0,1fr)_auto] sm:gap-6",
        className,
      )}
    >
      <div className="min-w-0">
        <p id={labelId} className="text-sm font-medium text-foreground">
          {label}
        </p>
        {description ? <div className="mt-0.5 text-sm text-muted-foreground">{description}</div> : null}
      </div>
      <div className="flex min-w-0 items-center gap-2 sm:justify-end">{control(labelId)}</div>
    </div>
  );
}

/** A quiet banner for sentences the product must say (locks, broken rows, rebuild). */
export function SettingsNotice({
  tone = "info",
  children,
  testId,
}: {
  tone?: "info" | "warning" | "danger" | "success";
  children: ReactNode;
  testId?: string;
}) {
  const toneClass = {
    info: "border-border bg-muted/40 text-foreground",
    warning: "border-amber-500/30 bg-amber-500/10 text-foreground",
    danger: "border-destructive/30 bg-destructive/10 text-foreground",
    success: "border-emerald-500/30 bg-emerald-500/10 text-foreground",
  }[tone];
  return (
    <div
      role={tone === "danger" ? "alert" : "note"}
      data-testid={testId}
      className={cn("rounded-md border px-3 py-2 text-sm", toneClass)}
    >
      {children}
    </div>
  );
}
