import { describe, expect, it } from 'vitest';
import {
  blockedRoles,
  isBlocked,
  judgeRole,
  nothingOffered,
  type RoleList,
  type Verdicts,
} from '../defaults-eligibility';
import { describeCreateFailure } from '../create-error';

const list = (models: RoleList['models'], loading = false): RoleList => ({ models, loading });

describe('judgeRole', () => {
  const chat = list([{ provider: 'openai', model: 'gpt-5-nano' }]);

  it('accepts a pair that is on the list, ignoring case', () => {
    expect(judgeRole('llm', { provider: 'OpenAI', model: 'GPT-5-nano' }, chat)).toBe('ok');
  });

  it('refuses a pair that fell off the list', () => {
    expect(judgeRole('llm', { provider: 'ollama', model: 'gemma3' }, chat)).toBe('not_eligible');
  });

  it('calls an empty pair unset, and a loading list loading', () => {
    expect(judgeRole('llm', {}, chat)).toBe('unset');
    expect(judgeRole('llm', { provider: 'a', model: 'b' }, list([], true))).toBe('loading');
  });

  it('wants a registered dimension for embeddings, from the list or the pick', () => {
    const missing = list([{ provider: 'ollama', model: 'embed', embedding_dimension: null }]);
    expect(judgeRole('embedding', { provider: 'ollama', model: 'embed' }, missing)).toBe('dimension_missing');
    expect(judgeRole('embedding', { provider: 'ollama', model: 'embed', dimension: 768 }, missing)).toBe('ok');
    const sized = list([{ provider: 'ollama', model: 'embed', embedding_dimension: 1024 }]);
    expect(judgeRole('embedding', { provider: 'ollama', model: 'embed' }, sized)).toBe('ok');
    const mistral = list([{ provider: 'mistral', model: 'mistral-embed', embedding_dimension: null }]);
    expect(judgeRole('embedding', { provider: 'mistral', model: 'mistral-embed' }, mistral)).toBe('ok');
    const zero = list([{ provider: 'mistral', model: 'mistral-embed', embedding_dimension: 0 }]);
    expect(judgeRole('embedding', { provider: 'mistral', model: 'mistral-embed' }, zero)).toBe('ok');
  });
});

describe('blocking', () => {
  const ok: Verdicts = { llm: 'ok', embedding: 'ok', vision: 'ok' };
  it('blocks on not_eligible and dimension_missing only', () => {
    expect(isBlocked(ok)).toBe(false);
    expect(isBlocked({ ...ok, vision: 'unset' })).toBe(false);
    expect(blockedRoles({ llm: 'not_eligible', embedding: 'dimension_missing', vision: 'ok' })).toEqual([
      'llm',
      'embedding',
    ]);
  });

  it('knows when nothing is offered at all', () => {
    const empty = list([]);
    expect(nothingOffered({ llm: empty, embedding: empty, vision: empty })).toBe(true);
    expect(nothingOffered({ llm: empty, embedding: list([], true), vision: empty })).toBe(false);
    expect(nothingOffered({ llm: list([{ provider: 'a', model: 'b' }]), embedding: empty, vision: empty })).toBe(false);
  });
});

describe('describeCreateFailure', () => {
  it('sends model refusals back to the models step', () => {
    for (const code of ['model_not_eligible', 'capability_mismatch', 'model_pair_incomplete', 'embedding_dimension_required', 'credential_missing']) {
      expect(describeCreateFailure({ code, message: 'x' }).step).toBe('models');
    }
  });

  it('keeps the server message and falls back for unknown codes', () => {
    expect(describeCreateFailure(Object.assign(new Error('Slug taken'), { code: 'slug_taken' }))).toEqual({
      title: 'Failed to create workspace',
      description: 'Slug taken',
      step: undefined,
    });
    expect(describeCreateFailure(null).description).toBe('Unknown error');
    expect(describeCreateFailure({ code: 'offer_unavailable' }).step).toBeUndefined();
  });
});
