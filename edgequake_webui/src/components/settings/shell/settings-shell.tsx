"use client";

/**
 * SPEC-175 — the one settings shell (LAW-175-8). Used by `/settings/*` and by the
 * `/w/{slug}/settings/*` deeplinks, so there is one nav, one search and one gate.
 *
 * Fail closed: gated content is not rendered until the server has answered, and a
 * section the caller may not open redirects to a page they may open (EC-175-34, EC-175-43).
 */

import { AccessBadge } from "@/components/settings/shell/access-badge";
import { MobileSettingsNav } from "@/components/settings/shell/settings-mobile-nav";
import { SettingsNav } from "@/components/settings/shell/settings-nav";
import { SettingsSearch } from "@/components/settings/shell/settings-search";
import { SettingsSectionRenderer } from "@/components/settings/sections";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsContext } from "@/hooks/use-settings-context";
import { canOpenSection, getSection, scopeHref, sectionFromPathname, sectionHref } from "@/lib/settings/registry";
import { rememberedAppPath } from "@/lib/settings/return-path";
import { ArrowLeft } from "lucide-react";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useState } from "react";

export function SettingsShell({ workspaceSlug = null }: { workspaceSlug?: string | null }) {
  const pathname = usePathname() ?? "/settings";
  const router = useRouter();
  const match = sectionFromPathname(pathname);
  const ctx = useSettingsContext(workspaceSlug);
  const { access, accessReady, accessError, workspace, workspaceState, administeredWorkspaces } = ctx;

  const section = match ? getSection(match.section) : null;
  const canonicalHref = match ? sectionHref(match.section, workspaceSlug) : null;
  // A workspace section waits for its workspace to resolve before the access gate decides.
  const workspaceLoading = section?.scope === "workspace" && workspaceState === "loading";
  const decided = !!section && accessReady && !workspaceLoading;
  const allowed = decided && canOpenSection(access, section);
  const redirectTarget = section?.scope === "workspace" ? sectionHref("models", workspaceSlug) : scopeHref("you");

  useEffect(() => {
    if (match?.bare && canonicalHref) router.replace(canonicalHref);
  }, [match?.bare, canonicalHref, router]);

  useEffect(() => {
    if (!decided || accessError || allowed) return;
    router.replace(redirectTarget);
  }, [decided, accessError, allowed, redirectTarget, router]);

  const [backHref, setBackHref] = useState("/");
  useEffect(() => setBackHref(rememberedAppPath() ?? "/"), []);

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden" data-testid="settings-shell">
      <aside
        aria-label="Settings navigation"
        className="hidden w-[248px] shrink-0 flex-col gap-4 overflow-y-auto border-r bg-muted/20 p-4 md:flex"
      >
        <Link
          href={backHref}
          className="inline-flex w-fit items-center gap-1.5 rounded-sm text-sm text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          data-testid="settings-back"
        >
          <ArrowLeft className="h-3.5 w-3.5" aria-hidden="true" />
          Back to app
        </Link>
        <div className="flex items-center justify-between gap-2">
          <p className="text-base font-semibold text-foreground">Settings</p>
          {accessReady ? <AccessBadge access={access} /> : null}
        </div>
        <SettingsSearch access={access} workspaceSlug={workspaceSlug} />
        {accessReady && match ? (
          <SettingsNav
            access={access}
            activeSection={match.section}
            workspaceSlug={workspaceSlug}
            workspace={workspace}
            workspaces={administeredWorkspaces}
          />
        ) : (
          <NavSkeleton />
        )}
      </aside>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col">
        <div className="space-y-3 border-b px-4 py-3 md:hidden">
          <div className="flex items-center justify-between gap-2">
            <Link href={backHref} className="inline-flex items-center gap-1.5 text-sm text-muted-foreground">
              <ArrowLeft className="h-3.5 w-3.5" aria-hidden="true" />
              Back to app
            </Link>
            {accessReady ? <AccessBadge access={access} /> : null}
          </div>
          <SettingsSearch access={access} workspaceSlug={workspaceSlug} />
          {accessReady && match ? (
            <MobileSettingsNav access={access} activeSection={match.section} workspaceSlug={workspaceSlug} />
          ) : null}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto">
          <main
            id="settings-main"
            aria-label="Settings content"
            className="mx-auto w-full max-w-[720px] px-4 py-8 sm:px-8 md:px-10"
            data-testid="settings-content"
          >
            {!section ? <UnknownSettingsPage /> : null}
            {section && !decided && !accessError ? <ContentSkeleton /> : null}
            {section && accessError ? <AccessErrorState /> : null}
            {allowed ? (
              <SettingsSectionRenderer id={section.id} workspaceSlug={workspaceSlug} />
            ) : null}
          </main>
        </div>
      </div>
    </div>
  );
}

function NavSkeleton() {
  return (
    <div className="space-y-2" aria-hidden="true" data-testid="settings-nav-skeleton">
      <Skeleton className="h-3 w-24" />
      <Skeleton className="h-7 w-full" />
      <Skeleton className="h-7 w-full" />
      <Skeleton className="h-7 w-full" />
    </div>
  );
}

function ContentSkeleton() {
  return (
    <div className="space-y-4" role="status" aria-live="polite" data-testid="settings-loading">
      <span className="sr-only">Loading settings</span>
      <Skeleton className="h-7 w-56" />
      <Skeleton className="h-4 w-96 max-w-full" />
      <Skeleton className="mt-6 h-40 w-full" />
    </div>
  );
}

function AccessErrorState() {
  return (
    <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 p-4 text-sm" data-testid="settings-access-error">
      <p className="font-medium">We could not confirm your settings access.</p>
      <p className="mt-1 text-muted-foreground">Reload the page. Nothing was changed.</p>
    </div>
  );
}

function UnknownSettingsPage() {
  return (
    <div className="space-y-2" data-testid="settings-unknown">
      <h1 tabIndex={-1} className="text-xl font-semibold">Settings page not found</h1>
      <p className="text-sm text-muted-foreground">
        <Link href={scopeHref("you")} className="underline underline-offset-2">Go to your settings</Link>
      </p>
    </div>
  );
}
