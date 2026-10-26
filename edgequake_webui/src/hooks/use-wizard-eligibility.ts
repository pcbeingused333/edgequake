'use client';

/**
 * SPEC-179 — judges the pair a wizard will save against the offer for its scope.
 * Reads `/models/eligible` (never the raw inventory), so it agrees with the pickers.
 */

import { useMemo } from 'react';
import { useChatFamilyModels, useScopedEligibleModels } from '@/hooks/use-model-policy';
import {
  isBlocked,
  judgeRole,
  nothingOffered,
  type ModelPair,
  type RoleList,
  type Verdicts,
  type WizardRole,
} from '@/lib/onboarding/defaults-eligibility';
import type { ModelDefaultsSlice } from '@/lib/onboarding/inherited-defaults';

export type WizardScope = 'workspace' | 'tenant' | 'deployment';

export interface WizardEligibilityInput {
  scope: WizardScope;
  id: string | null | undefined;
  /** True when the draft inherits (no custom picks). */
  useDefaults: boolean;
  inherited: ModelDefaultsSlice;
  llm?: ModelPair;
  embedding?: ModelPair;
  vision?: ModelPair;
}

export interface WizardEligibility {
  verdicts: Verdicts;
  /** The pair each verdict judged. Shown by the notice. */
  pairs: Record<WizardRole, ModelPair>;
  blocked: boolean;
  nothingOffered: boolean;
  isLoading: boolean;
  isError: boolean;
}

export function useWizardEligibility(input: WizardEligibilityInput): WizardEligibility {
  const chat = useChatFamilyModels({ scope: input.scope, id: input.id });
  const embeddingQuery = useScopedEligibleModels({ scope: input.scope, id: input.id, usage: 'embedding' });
  const visionQuery = useScopedEligibleModels({ scope: input.scope, id: input.id, usage: 'vlm' });
  const { useDefaults, inherited, llm, embedding, vision } = input;

  return useMemo(() => {
    const pairs: Record<WizardRole, ModelPair> = useDefaults
      ? {
          llm: { provider: inherited.defaultLlmProvider, model: inherited.defaultLlmModel },
          embedding: {
            provider: inherited.defaultEmbeddingProvider,
            model: inherited.defaultEmbeddingModel,
            dimension: inherited.defaultEmbeddingDimension,
          },
          vision: { provider: inherited.defaultVisionProvider, model: inherited.defaultVisionModel },
        }
      : { llm: llm ?? {}, embedding: embedding ?? {}, vision: vision ?? {} };
    const lists: Record<WizardRole, RoleList> = {
      llm: { models: chat.models, loading: chat.isLoading },
      embedding: { models: embeddingQuery.data?.models ?? [], loading: embeddingQuery.isLoading },
      vision: { models: visionQuery.data?.models ?? [], loading: visionQuery.isLoading },
    };
    const verdicts: Verdicts = {
      llm: judgeRole('llm', pairs.llm, lists.llm),
      embedding: judgeRole('embedding', pairs.embedding, lists.embedding),
      vision: judgeRole('vision', pairs.vision, lists.vision),
    };
    const isError = chat.isError || embeddingQuery.isError || visionQuery.isError;
    return {
      verdicts,
      pairs,
      blocked: isBlocked(verdicts),
      nothingOffered: !isError && nothingOffered(lists),
      isLoading: Object.values(lists).some((list) => list.loading),
      isError,
    };
  }, [
    useDefaults,
    inherited,
    llm,
    embedding,
    vision,
    chat.models,
    chat.isLoading,
    chat.isError,
    embeddingQuery.data,
    embeddingQuery.isLoading,
    embeddingQuery.isError,
    visionQuery.data,
    visionQuery.isLoading,
    visionQuery.isError,
  ]);
}
