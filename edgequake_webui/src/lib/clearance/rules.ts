/** The only place the Access screen becomes allow and deny rules, and back again. */

import type { RuleView } from "./types";

export type Ceiling = "public" | "internal" | "confidential" | "restricted";

export const CEILINGS: readonly Ceiling[] = ["public", "internal", "confidential", "restricted"];

export interface ProfileDraft {
  ceiling: Ceiling;
  departmentIds: string[];
  exceptIds: string[];
}

export interface ClearanceRuleBody {
  effect: "allow" | "deny";
  match_kind: "classification_at_most" | "any_of" | "untagged";
  rank?: number;
  tag_ids?: string[];
}

const RANK: Record<Ceiling, number> = {
  public: 1,
  internal: 2,
  confidential: 3,
  restricted: 4,
};

/** New profiles start at Internal, the usual ceiling for staff. */
export const DEFAULT_CEILING: Ceiling = "internal";

export function emptyDraft(): ProfileDraft {
  return { ceiling: DEFAULT_CEILING, departmentIds: [], exceptIds: [] };
}

export function rulesFromDraft(draft: ProfileDraft): ClearanceRuleBody[] {
  const rules: ClearanceRuleBody[] = [
    {
      effect: "allow",
      match_kind: "classification_at_most",
      rank: RANK[draft.ceiling],
    },
  ];
  if (draft.departmentIds.length > 0) {
    rules.push({
      effect: "allow",
      match_kind: "any_of",
      tag_ids: draft.departmentIds,
    });
  }
  if (draft.exceptIds.length > 0) {
    rules.push({
      effect: "deny",
      match_kind: "any_of",
      tag_ids: draft.exceptIds,
    });
  }
  return rules;
}

/** Inverse of `rulesFromDraft`: read a saved profile back into the editor. */
export function draftFromRules(rules: RuleView[]): ProfileDraft {
  const draft = emptyDraft();
  const ceiling = rules.find(
    (rule) => rule.effect === "allow" && rule.match_kind === "classification_at_most",
  );
  if (ceiling?.rank != null) {
    draft.ceiling = ceilingForRank(ceiling.rank);
  }
  draft.departmentIds = unionTags(rules, "allow");
  draft.exceptIds = unionTags(rules, "deny");
  return draft;
}

/** Map a stored rank to the nearest editable ceiling. Unverified (5) reads as Restricted. */
export function ceilingForRank(rank: number): Ceiling {
  if (rank <= 1) return "public";
  if (rank === 2) return "internal";
  if (rank === 3) return "confidential";
  return "restricted";
}

/** Order-insensitive equality for id lists. */
export function sameIds(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  const set = new Set(a);
  return b.every((id) => set.has(id));
}

export function sameDraft(a: ProfileDraft, b: ProfileDraft): boolean {
  return (
    a.ceiling === b.ceiling &&
    sameIds(a.departmentIds, b.departmentIds) &&
    sameIds(a.exceptIds, b.exceptIds)
  );
}

function unionTags(rules: RuleView[], effect: "allow" | "deny"): string[] {
  const ids = new Set<string>();
  for (const rule of rules) {
    if (rule.effect === effect && rule.match_kind === "any_of") {
      rule.tag_ids.forEach((id) => ids.add(id));
    }
  }
  return [...ids];
}
