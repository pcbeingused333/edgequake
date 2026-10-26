import { describe, expect, it } from 'vitest';
import {
  buildTenantModelPayload,
  buildWorkspaceIngestPayload,
  buildWorkspaceModelPayload,
  buildWorkspaceUpdatePayload,
  normalizeModelFullId,
} from '../model-payload';
import { EMPTY_WIZARD_DRAFT } from '../wizard-state';

describe('model-payload', () => {
  it('omits all model fields when using server defaults (tenant)', () => {
    expect(
      buildTenantModelPayload({
        useServerDefaults: true,
        llm: { provider: 'ollama', model: 'gemma4:latest' },
        embedding: { provider: 'openai', model: 'text-embedding-3-small', dimension: 1536 },
        vision: { provider: 'ollama', model: 'gemma4:latest' },
      }),
    ).toEqual({});
  });

  it('includes slash-based overrides when not using server defaults (tenant)', () => {
    expect(
      buildTenantModelPayload({
        useServerDefaults: false,
        llm: { provider: 'ollama', model: 'gemma4:latest' },
        embedding: { provider: 'openai', model: 'text-embedding-3-small', dimension: 1536 },
        vision: { provider: 'openai', model: 'gpt-4o' },
      }),
    ).toEqual({
      default_llm_provider: 'ollama',
      default_llm_model: 'gemma4:latest',
      default_embedding_provider: 'openai',
      default_embedding_model: 'text-embedding-3-small',
      default_embedding_dimension: 1536,
      default_vision_llm_provider: 'openai',
      default_vision_llm_model: 'gpt-4o',
    });
  });

  it('omits workspace model fields when using server defaults', () => {
    expect(
      buildWorkspaceModelPayload({
        useServerDefaults: true,
        llm: { provider: 'a', model: 'b' },
      }),
    ).toEqual({});
  });

  it('builds workspace overrides', () => {
    expect(
      buildWorkspaceModelPayload({
        useServerDefaults: false,
        llm: { provider: 'ollama', model: 'x' },
        embedding: { provider: 'openai', model: 'e', dimension: 768 },
        vision: { provider: 'ollama', model: 'v' },
      }),
    ).toEqual({
      llm_provider: 'ollama',
      llm_model: 'x',
      embedding_provider: 'openai',
      embedding_model: 'e',
      vision_llm_provider: 'ollama',
      vision_llm_model: 'v',
    });
  });

  it('normalizes colon legacy ids to slash', () => {
    expect(normalizeModelFullId('ollama:gemma4:latest')).toEqual({
      provider: 'ollama',
      model: 'gemma4:latest',
      fullId: 'ollama/gemma4:latest',
    });
    expect(normalizeModelFullId('openai/gpt-4o')).toEqual({
      provider: 'openai',
      model: 'gpt-4o',
      fullId: 'openai/gpt-4o',
    });
  });

  it('clears model overrides when useServerDefaults on update (EC-101-19)', () => {
    expect(
      buildWorkspaceUpdatePayload({
        useServerDefaults: true,
        llm: { provider: 'ollama', model: 'x' },
        embedding: { provider: 'openai', model: 'e', dimension: 768 },
        vision: { provider: 'ollama', model: 'v' },
        pdfParserBackend: 'vision',
        extractionLanguage: null,
        entityTypes: ['PERSON'],
        entityTypesStrict: true,
      }),
    ).toEqual({
      llm_model: '',
      llm_provider: '',
      embedding_model: '',
      embedding_provider: '',
      embedding_dimension: 0,
      vision_llm_model: '',
      vision_llm_provider: '',
      llm_roles: {},
    });
  });

  it('builds update payload with overrides', () => {
    expect(
      buildWorkspaceUpdatePayload({
        useServerDefaults: false,
        llm: { provider: 'ollama', model: 'gemma4:latest' },
        embedding: { provider: 'ollama', model: 'embeddinggemma', dimension: 768 },
        vision: { provider: 'ollama', model: 'gemma4:latest' },
        pdfParserBackend: 'edgeparse',
        extractionLanguage: 'Chinese',
        chunkingMode: 'fixed',
        chunkTokenSize: 1200,
        chunkOverlapTokenSize: 100,
        entityTypes: ['PERSON', 'ORGANIZATION'],
        entityTypesStrict: false,
        entityTypeColors: { PERSON: '#112233' },
      }),
    ).toMatchObject({
      llm_provider: 'ollama',
      llm_model: 'gemma4:latest',
      embedding_provider: 'ollama',
      embedding_model: 'embeddinggemma',
      vision_llm_provider: '',
      vision_llm_model: '',
      llm_roles: { vlm: { provider: 'ollama', model: 'gemma4:latest' } },
      pdf_parser_backend: 'edgeparse',
      entity_types_strict: false,
      extraction_language: 'Chinese',
      chunking_mode: 'fixed',
      chunk_token_size: 1200,
      chunk_overlap_token_size: 100,
      extract_budget_mode: 'inherit',
      entity_type_colors: { PERSON: '#112233' },
    });
  });

  it('builds update payload with LightRAG extract budget', () => {
    expect(
      buildWorkspaceUpdatePayload({
        useServerDefaults: false,
        llm: { provider: 'ollama', model: 'gemma4:latest' },
        embedding: { provider: 'ollama', model: 'embeddinggemma', dimension: 768 },
        vision: { provider: 'ollama', model: 'gemma4:latest' },
        pdfParserBackend: 'vision',
        extractionLanguage: null,
        extractBudgetMode: 'custom',
        extractMaxEntities: 40,
        extractMaxRecords: 100,
        entityTypes: ['PERSON'],
        entityTypesStrict: true,
      }),
    ).toMatchObject({
      extract_budget_mode: 'custom',
      extract_max_entities: 40,
      extract_max_records: 100,
    });
  });

  it('sparse PUT emits only changed keys (language must not promote models)', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: true,
      llm: { provider: 'ollama', model: 'gemma4:latest' },
      embedding: { provider: 'ollama', model: 'embeddinggemma', dimension: 768 },
      vision: { provider: 'ollama', model: 'gemma4:latest' },
      pdfParserBackend: 'none',
      extractionLanguage: 'Chinese',
      entityTypes: ['PERSON'],
      entityTypesStrict: true,
      changedKeys: ['extractionLanguage'],
    });
    expect(payload).toEqual({
      extraction_language: 'Chinese',
    });
  });

  it('sparse PUT with inherit models only clears model fields', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: true,
      pdfParserBackend: 'none',
      extractionLanguage: null,
      entityTypes: ['PERSON'],
      entityTypesStrict: true,
      changedKeys: ['llm', 'embedding', 'vision'],
    });
    expect(payload).toEqual({
      llm_model: '',
      llm_provider: '',
      embedding_model: '',
      embedding_provider: '',
      embedding_dimension: 0,
      vision_llm_model: '',
      vision_llm_provider: '',
      llm_roles: {},
    });
    expect(payload).not.toHaveProperty('entity_types');
    expect(payload).not.toHaveProperty('relation_types');
  });
});

describe('buildWorkspaceIngestPayload', () => {
  it('omits inherit-equivalent fields on create', () => {
    const payload = buildWorkspaceIngestPayload(
      {
        ...EMPTY_WIZARD_DRAFT,
        pdfParserBackend: 'none',
        chunkingMode: null,
        extractBudgetMode: null,
        visionExtractImages: true,
        visionExtractCharts: true,
        visionExtractFigures: true,
      },
      'create',
    );
    expect(payload.pdf_parser_backend).toBeUndefined();
    expect(payload.chunking_mode).toBeUndefined();
    expect(payload.extract_budget_mode).toBeUndefined();
    expect(payload.vision_extract_images).toBeUndefined();
  });

  it('sends create overrides for pdf, chunking, budget, and vision extract', () => {
    const payload = buildWorkspaceIngestPayload(
      {
        ...EMPTY_WIZARD_DRAFT,
        pdfParserBackend: 'edgeparse',
        chunkingMode: 'fixed',
        chunkTokenSize: 800,
        chunkOverlapTokenSize: 50,
        extractBudgetMode: 'custom',
        extractMaxEntities: 20,
        extractMaxRecords: 50,
        visionExtractImages: false,
        extractionLanguage: 'French',
      },
      'create',
    );
    expect(payload.pdf_parser_backend).toBe('edgeparse');
    expect(payload.chunking_mode).toBe('fixed');
    expect(payload.chunk_token_size).toBe(800);
    expect(payload.extract_max_entities).toBe(20);
    expect(payload.vision_extract_images).toBe(false);
    expect(payload.extraction_language).toBe('French');
  });

  it('sends inherit clears on update', () => {
    const payload = buildWorkspaceIngestPayload(
      {
        ...EMPTY_WIZARD_DRAFT,
        pdfParserBackend: 'none',
        chunkingMode: 'inherit',
        extractBudgetMode: 'inherit',
      },
      'update',
    );
    expect(payload.pdf_parser_backend).toBe('none');
    expect(payload.chunking_mode).toBe('inherit');
    expect(payload.extract_budget_mode).toBe('inherit');
  });

  it('a vision save keeps the other saved roles (SPEC-163)', () => {
    const payload = buildWorkspaceUpdatePayload({
      useServerDefaults: false,
      vision: { provider: 'openai', model: 'gpt-4o' },
      savedLlmRoles: {
        extract: { provider: 'openai', model: 'gpt-5-nano' },
        vlm: { provider: 'mistral', model: 'pixtral-large-latest' },
      },
      pdfParserBackend: 'none',
      extractionLanguage: null,
      entityTypes: ['PERSON'],
      entityTypesStrict: true,
      changedKeys: ['vision'],
    });
    expect(payload.llm_roles).toEqual({
      extract: { provider: 'openai', model: 'gpt-5-nano' },
      vlm: { provider: 'openai', model: 'gpt-4o' },
    });
  });
});
