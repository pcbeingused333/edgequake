import { describe, expect, it } from 'vitest';
import {
  prefillReconfigureFromWorkspace,
  resolveHydratedUseServerDefaults,
} from '../reconfigure-from-workspace';
import { buildWorkspaceUpdatePayload } from '../model-payload';
import type { Workspace } from '@/types';

function baseWorkspace(overrides: Partial<Workspace> = {}): Workspace {
  return {
    id: 'ws-1',
    tenant_id: 't-1',
    name: 'Demo',
    slug: 'demo',
    description: null,
    is_active: true,
    created_at: '',
    updated_at: '',
    llm_provider: 'ollama',
    llm_model: 'gemma4:latest',
    llm_full_id: 'ollama/gemma4:latest',
    embedding_provider: 'ollama',
    embedding_model: 'embeddinggemma',
    embedding_dimension: 768,
    ...overrides,
  } as Workspace;
}

describe('resolveHydratedUseServerDefaults', () => {
  it('forces false when workspace has overrides (draft flag trap)', () => {
    expect(
      resolveHydratedUseServerDefaults({
        prefillAdvancedOpen: true,
        prefillUseServerDefaults: false,
        hasPrefillPicks: true,
        storedUseServerDefaults: true,
      }),
    ).toBe(false);
  });

  it('keeps stored true when workspace truly uses defaults', () => {
    expect(
      resolveHydratedUseServerDefaults({
        prefillAdvancedOpen: false,
        prefillUseServerDefaults: true,
        hasPrefillPicks: false,
        storedUseServerDefaults: true,
      }),
    ).toBe(true);
  });
});

describe('reconfigure persist payload', () => {
  it('sends mistral overrides when useServerDefaults is false after picker', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: false,
      llm: { provider: 'mistral', model: 'mistral-small-latest' },
      embedding: {
        provider: 'mistral',
        model: 'mistral-embed',
        dimension: 1024,
      },
      vision: undefined,
      pdfParserBackend: 'none',
      extractionLanguage: null,
      entityTypes: ['PERSON'],
      entityTypesStrict: true,
    });
    expect(payload).toMatchObject({
      llm_provider: 'mistral',
      llm_model: 'mistral-small-latest',
      embedding_provider: 'mistral',
      embedding_model: 'mistral-embed',
    });
    // The server copies the dimension from the model card; the client does not send one.
    expect(payload).not.toHaveProperty('embedding_dimension');
    expect(payload.llm_model).not.toBe('');
  });

  it('prefill opens Advanced when workspace has concrete LLM picks', () => {
    const prefill = prefillReconfigureFromWorkspace(
      baseWorkspace({
        llm_resolution_source: 'workspace',
        embedding_resolution_source: 'workspace',
      }),
    );
    expect(prefill.advancedOpen).toBe(true);
    expect(prefill.draft.useServerDefaults).toBe(false);
    expect(prefill.llm?.provider).toBe('ollama');
    expect(prefill.draft.llmPick?.provider).toBe('ollama');
    expect(prefill.draft.advancedOpen).toBe(true);
    expect(prefill.draft.llmPick?.provider).toBe('ollama');
    expect(prefill.draft.advancedOpen).toBe(true);
  });

  it('prefill treats painted tenant models as inherit (not overrides)', () => {
    const prefill = prefillReconfigureFromWorkspace(
      baseWorkspace({
        llm_resolution_source: 'tenant',
        embedding_resolution_source: 'tenant',
      }),
    );
    expect(prefill.advancedOpen).toBe(false);
    expect(prefill.draft.useServerDefaults).toBe(true);
    expect(prefill.llm).toBeUndefined();
    expect(prefill.embedding).toBeUndefined();
    expect(prefill.draft.llmPick).toBeUndefined();
    expect(prefill.draft.advancedOpen).toBe(false);
    expect(prefill.draft.llmPick).toBeUndefined();
    expect(prefill.draft.embeddingPick).toBeUndefined();
  });

  it('prefill seeds relation defaults when kg_schema_preset names a domain', () => {
    const prefill = prefillReconfigureFromWorkspace(
      baseWorkspace({
        kg_schema_preset: 'manufacturing',
        entity_types: undefined,
        relation_types: [],
      }),
    );
    expect(prefill.draft.relationTypes).toContain('PART_OF');
    expect(prefill.draft.relationEdges.some((e) => e.relation === 'HAS_DEFECT')).toBe(
      true,
    );
    expect(prefill.draft.kgSchemaPreset).toBe('manufacturing');
    expect(prefill.snapshot.relationTypes).toEqual(prefill.draft.relationTypes);
    expect(prefill.snapshot.relationEdges).toEqual(prefill.draft.relationEdges);
  });

  it('prefill keeps free-form relations when no named domain preset', () => {
    const prefill = prefillReconfigureFromWorkspace(
      baseWorkspace({
        kg_schema_preset: undefined,
        entity_types: ['PERSON'],
        relation_types: [],
      }),
    );
    expect(prefill.draft.relationTypes).toEqual([]);
  });
});

describe('embedding live catalog onChange contract', () => {
  it('buildWorkspaceUpdatePayload leaves the dimension to the server when the catalog misses', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: false,
      llm: { provider: 'mistral', model: 'mistral-small-latest' },
      embedding: {
        provider: 'mistral',
        model: 'mistral-embed-live',
        dimension: 0,
      },
      pdfParserBackend: 'none',
      extractionLanguage: null,
      entityTypes: [],
      entityTypesStrict: true,
    });
    expect(payload.embedding_model).toBe('mistral-embed-live');
    expect(payload).not.toHaveProperty('embedding_dimension');
  });
});

describe('SPEC-160 reconfigure Decision payload', () => {
  it('emits extraction mode and decision engine when those keys change', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: true,
      pdfParserBackend: 'none',
      extractionLanguage: null,
      entityTypes: [],
      entityTypesStrict: true,
      decisionEnabled: true,
      decisionModel: 'tev1:latest',
      decisionPreset: 'recall',
      decisionPackSize: '6',
      extractionMode: 'decision',
      changedKeys: ['decisionEngine', 'extractionMode'],
    });
    expect(payload).toMatchObject({
      decision_enabled: true,
      decision_model: 'tev1:latest',
      decision_gate_preset: 'recall',
      decision_pack_size: 6,
      extraction_mode: 'decision',
    });
    expect(payload.llm_model).toBeUndefined();
  });
});
