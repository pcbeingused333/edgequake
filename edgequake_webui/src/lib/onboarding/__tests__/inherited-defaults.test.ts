import { describe, expect, it } from 'vitest';
import {
  formatInheritModelLabel,
  pickDefaultWorkspaceLanguage,
  resolveInheritedModelDefaults,
  type ModelDefaultsSlice,
} from '../inherited-defaults';

const server: ModelDefaultsSlice = {
  defaultLlmProvider: 'ollama',
  defaultLlmModel: 'gemma4:latest',
  defaultEmbeddingProvider: 'ollama',
  defaultEmbeddingModel: 'embeddinggemma',
  defaultVisionProvider: 'ollama',
  defaultVisionModel: 'gemma4:latest',
};

describe('resolveInheritedModelDefaults', () => {
  it('falls back to server when tenant has no model fields', () => {
    const resolved = resolveInheritedModelDefaults({}, server, null);
    expect(resolved.source).toBe('server');
    expect(resolved.defaultLlmModel).toBe('gemma4:latest');
    expect(resolved.defaultEmbeddingModel).toBe('embeddinggemma');
    expect(resolved.hasConfiguredDefaults).toBe(true);
  });

  it('prefers tenant overrides when present', () => {
    const resolved = resolveInheritedModelDefaults(
      {
        default_llm_provider: 'mistral',
        default_llm_model: 'mistral-small-latest',
        default_embedding_provider: 'mistral',
        default_embedding_model: 'mistral-embed',
        default_vision_llm_provider: 'mistral',
        default_vision_llm_model: 'mistral-small-latest',
      },
      server,
      'French',
    );
    expect(resolved.source).toBe('tenant');
    expect(resolved.defaultLlmProvider).toBe('mistral');
    expect(resolved.defaultLlmModel).toBe('mistral-small-latest');
    expect(resolved.defaultEmbeddingModel).toBe('mistral-embed');
    expect(resolved.defaultVisionModel).toBe('mistral-small-latest');
    expect(resolved.extractionLanguage).toBe('French');
  });

  it('mixes tenant LLM with server embedding when embedding unset', () => {
    const resolved = resolveInheritedModelDefaults(
      {
        default_llm_provider: 'openai',
        default_llm_model: 'gpt-5-nano',
      },
      server,
    );
    expect(resolved.source).toBe('tenant');
    expect(resolved.defaultLlmModel).toBe('gpt-5-nano');
    expect(resolved.defaultEmbeddingModel).toBe('embeddinggemma');
  });
});

describe('pickDefaultWorkspaceLanguage', () => {
  it('prefers slug=default then Default Workspace name', () => {
    expect(
      pickDefaultWorkspaceLanguage([
        { slug: 'other', name: 'Other', extraction_language: 'German' },
        { slug: 'default', name: 'Default Workspace', extraction_language: 'French' },
      ]),
    ).toBe('French');
    expect(
      pickDefaultWorkspaceLanguage([
        { slug: 'main', name: 'Default Workspace', extraction_language: 'Chinese' },
      ]),
    ).toBe('Chinese');
  });

  it('returns null when unset', () => {
    expect(pickDefaultWorkspaceLanguage([{ slug: 'default', name: 'Default Workspace' }])).toBe(
      null,
    );
  });
});

describe('formatInheritModelLabel', () => {
  it('includes tenant prefix and resolved id', () => {
    expect(
      formatInheritModelLabel({
        source: 'tenant',
        provider: 'ollama',
        model: 'gemma4:latest',
      }),
    ).toBe('Organization default (ollama/gemma4:latest)');
  });

  it('includes server prefix and resolved id', () => {
    expect(
      formatInheritModelLabel({
        source: 'server',
        provider: 'openai',
        model: 'text-embedding-3-small',
      }),
    ).toBe('Server default (openai/text-embedding-3-small)');
  });
});
