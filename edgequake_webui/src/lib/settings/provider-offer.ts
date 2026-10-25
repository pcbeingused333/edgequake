/**
 * SPEC-177 — pure draft helpers for the provider offer editor.
 * The server payload is the source of the mode. These functions only
 * describe the draft the user is editing.
 */

export type OfferMode = "unconstrained" | "constrained";

export interface OfferProvider {
  slug: string;
  displayName: string;
  enabled: boolean;
  modelCount: number;
}

export interface OfferDraft {
  mode: OfferMode;
  /** Enabled slugs. Ignored by the server when mode is unconstrained. */
  slugs: string[];
}

export function draftFromProviders(mode: OfferMode, providers: readonly { slug: string; enabled: boolean }[]): OfferDraft {
  return {
    mode,
    slugs: providers.filter((row) => row.enabled).map((row) => row.slug).sort(),
  };
}

export function sameDraft(a: OfferDraft, b: OfferDraft): boolean {
  if (a.mode !== b.mode) return false;
  if (a.slugs.length !== b.slugs.length) return false;
  return a.slugs.every((slug, index) => slug === b.slugs[index]);
}

/** Model ticks for providers the draft turned off are omitted from the save. */
export function slugsForSave(draft: OfferDraft): string[] {
  return draft.mode === "constrained" ? draft.slugs : [];
}

export function toggleSlug(draft: OfferDraft, slug: string, providers: readonly string[]): OfferDraft {
  const base = draft.mode === "unconstrained" ? [...providers] : [...draft.slugs];
  const next = new Set(base);
  if (next.has(slug)) next.delete(slug);
  else next.add(slug);
  return { mode: "constrained", slugs: [...next].sort() };
}

export function lead(parent: "tenant" | "workspace", mode: OfferMode): string {
  if (parent === "tenant") {
    return mode === "unconstrained"
      ? "People in this organization can use every provider this deployment offers."
      : "Only the providers enabled here appear in workspaces.";
  }
  return mode === "unconstrained"
    ? "This workspace can use every provider the organization offers."
    : "Only the providers enabled here appear in this workspace.";
}
