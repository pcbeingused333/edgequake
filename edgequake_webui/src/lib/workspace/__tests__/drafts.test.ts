import { describe, expect, it } from 'vitest';
import type { Workspace } from '@/types';
import {
  getWorkspaceEmbeddingSelection,
  getWorkspaceLlmSelection,
  getWorkspaceVisionSelection,
  isWorkspaceResolutionSource,
} from '../drafts';

function ws(overrides: Partial<Workspace> = {}): Workspace {
  return {
    id: 'ws-1',
    tenant_id: 't-1',
    name: 'Demo',
    slug: 'demo',
    is_active: true,
    created_at: '',
    llm_provider: 'ollama',
    llm_model: 'gemma4:latest',
    embedding_provider: 'ollama',
    embedding_model: 'embeddinggemma',
    embedding_dimension: 768,
    ...overrides,
  } as Workspace;
}

describe('isWorkspaceResolutionSource', () => {
  it('is true only for workspace', () => {
    expect(isWorkspaceResolutionSource('workspace')).toBe(true);
    expect(isWorkspaceResolutionSource('tenant')).toBe(false);
    expect(isWorkspaceResolutionSource('env')).toBe(false);
    expect(isWorkspaceResolutionSource(undefined)).toBe(false);
  });
});

describe('getWorkspaceLlmSelection overridesOnly', () => {
  it('returns painted model when not filtering', () => {
    expect(getWorkspaceLlmSelection(ws({ llm_resolution_source: 'tenant' }))?.model).toBe(
      'gemma4:latest',
    );
  });

  it('returns undefined for tenant/env when overridesOnly', () => {
    expect(
      getWorkspaceLlmSelection(ws({ llm_resolution_source: 'tenant' }), {
        overridesOnly: true,
      }),
    ).toBeUndefined();
  });

  it('returns pick when source is workspace', () => {
    expect(
      getWorkspaceLlmSelection(ws({ llm_resolution_source: 'workspace' }), {
        overridesOnly: true,
      })?.provider,
    ).toBe('ollama');
  });
});

describe('getWorkspaceVisionSelection overridesOnly', () => {
  it('ignores painted legacy fields whose source is not the workspace', () => {
    expect(
      getWorkspaceVisionSelection(
        ws({
          vision_llm_provider: 'mistral',
          vision_llm_model: 'mistral-small-latest',
          vision_llm_resolution_source: 'tenant',
        }),
        { overridesOnly: true },
      ),
    ).toBeUndefined();
  });

  it('keeps a painted pair when the source is the workspace', () => {
    expect(
      getWorkspaceVisionSelection(
        ws({
          vision_llm_provider: 'mistral',
          vision_llm_model: 'mistral-small-latest',
          vision_llm_resolution_source: 'workspace',
        }),
        { overridesOnly: true },
      )?.model,
    ).toBe('mistral-small-latest');
  });

  it('keeps a canonical llm_roles.vlm row stored on the workspace', () => {
    expect(
      getWorkspaceVisionSelection(
        ws({
          llm_roles: { vlm: { provider: 'ollama', model: 'qwen3' } },
        }),
        { overridesOnly: true },
      )?.fullId,
    ).toBe('ollama/qwen3');
  });
});

describe('getWorkspaceEmbeddingSelection overridesOnly', () => {
  it('returns undefined for painted tenant embedding', () => {
    expect(
      getWorkspaceEmbeddingSelection(ws({ embedding_resolution_source: 'tenant' }), {
        overridesOnly: true,
      }),
    ).toBeUndefined();
  });
});
