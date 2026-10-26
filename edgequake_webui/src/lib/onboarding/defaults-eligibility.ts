/**
 * SPEC-179 — does the model pair a wizard is about to save still belong to the offer?
 * Pure. The server judges again on save; this only keeps the wizard from walking
 * a user into a refusal they could have been told about on the models step.
 */

export type WizardRole = 'llm' | 'embedding' | 'vision';

export type RoleStatus = 'ok' | 'not_eligible' | 'unset' | 'dimension_missing' | 'loading';

export interface ModelPair {
  provider?: string;
  model?: string;
  /** Embedding only. 0 or absent means the dimension is not registered. */
  dimension?: number;
}

export interface EligibleLike {
  provider: string;
  model: string;
  embedding_dimension?: number | null;
}

export interface RoleList {
  models: readonly EligibleLike[];
  loading: boolean;
}

const same = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();

export function judgeRole(role: WizardRole, pair: ModelPair, list: RoleList): RoleStatus {
  if (!pair.provider?.trim() || !pair.model?.trim()) return 'unset';
  if (list.loading) return 'loading';
  const match = list.models.find(
    (m) => same(m.provider, pair.provider as string) && same(m.model, pair.model as string),
  );
  if (!match) return 'not_eligible';
  if (role === 'embedding') {
    const dimension = match.embedding_dimension ?? pair.dimension ?? 0;
    if (!(dimension > 0)) return 'dimension_missing';
  }
  return 'ok';
}

export type Verdicts = Record<WizardRole, RoleStatus>;

/** Statuses that must stop the wizard. `unset` is the existing "defaults incomplete" path. */
export const BLOCKING: readonly RoleStatus[] = ['not_eligible', 'dimension_missing'];

export const blockedRoles = (verdicts: Verdicts): WizardRole[] =>
  (Object.keys(verdicts) as WizardRole[]).filter((role) => BLOCKING.includes(verdicts[role]));

export const isBlocked = (verdicts: Verdicts): boolean => blockedRoles(verdicts).length > 0;

/** Nothing is offered at all: every list loaded and empty. Different fix than a stale pick. */
export function nothingOffered(lists: Record<WizardRole, RoleList>): boolean {
  const all = Object.values(lists);
  return all.every((list) => !list.loading) && all.every((list) => list.models.length === 0);
}

export const ROLE_LABEL: Record<WizardRole, string> = {
  llm: 'LLM',
  embedding: 'Embedding',
  vision: 'Vision',
};
