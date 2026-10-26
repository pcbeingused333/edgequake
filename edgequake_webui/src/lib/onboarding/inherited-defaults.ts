/**
 * SPEC-101 — Pure tenant → server inheritance merge for create-workspace defaults.
 */

export type DefaultsSource = 'tenant' | 'server';

export interface ModelDefaultsSlice {
  defaultLlmProvider?: string;
  defaultLlmModel?: string;
  defaultEmbeddingProvider?: string;
  defaultEmbeddingModel?: string;
  defaultEmbeddingDimension?: number;
  defaultVisionProvider?: string;
  defaultVisionModel?: string;
}

export interface ResolvedInheritedDefaults extends ModelDefaultsSlice {
  source: DefaultsSource;
  hasConfiguredDefaults: boolean;
  /** Prefill for extraction language (from tenant Default Workspace when present). */
  extractionLanguage: string | null;
}

function pick(
  tenantVal: string | undefined | null,
  serverVal: string | undefined | null,
): string | undefined {
  const t = typeof tenantVal === 'string' ? tenantVal.trim() : '';
  if (t.length > 0) return t;
  const s = typeof serverVal === 'string' ? serverVal.trim() : '';
  return s.length > 0 ? s : undefined;
}

export interface TenantDefaultsInput {
  default_llm_provider?: string | null;
  default_llm_model?: string | null;
  default_embedding_provider?: string | null;
  default_embedding_model?: string | null;
  default_embedding_dimension?: number | null;
  default_vision_llm_provider?: string | null;
  default_vision_llm_model?: string | null;
}

/**
 * Merge tenant model fields over server defaults.
 * Source is `tenant` when any tenant model field contributed; otherwise `server`.
 */
export function resolveInheritedModelDefaults(
  tenant: TenantDefaultsInput | null | undefined,
  server: ModelDefaultsSlice,
  extractionLanguage: string | null = null,
): ResolvedInheritedDefaults {
  const llmProvider = pick(tenant?.default_llm_provider, server.defaultLlmProvider);
  const llmModel = pick(tenant?.default_llm_model, server.defaultLlmModel);
  const embProvider = pick(tenant?.default_embedding_provider, server.defaultEmbeddingProvider);
  const embModel = pick(tenant?.default_embedding_model, server.defaultEmbeddingModel);
  const visionProvider = pick(
    tenant?.default_vision_llm_provider,
    server.defaultVisionProvider ?? server.defaultLlmProvider,
  );
  const visionModel = pick(
    tenant?.default_vision_llm_model,
    server.defaultVisionModel ?? server.defaultLlmModel,
  );

  const tenantContributed = Boolean(
    pick(tenant?.default_llm_provider, null) ||
      pick(tenant?.default_llm_model, null) ||
      pick(tenant?.default_embedding_provider, null) ||
      pick(tenant?.default_embedding_model, null) ||
      pick(tenant?.default_vision_llm_provider, null) ||
      pick(tenant?.default_vision_llm_model, null),
  );

  const hasConfiguredDefaults = Boolean(llmModel && embModel);

  return {
    source: tenantContributed ? 'tenant' : 'server',
    hasConfiguredDefaults,
    defaultLlmProvider: llmProvider,
    defaultLlmModel: llmModel,
    defaultEmbeddingProvider: embProvider,
    defaultEmbeddingModel: embModel,
    defaultEmbeddingDimension:
      typeof tenant?.default_embedding_dimension === 'number' &&
      tenant.default_embedding_dimension > 0
        ? tenant.default_embedding_dimension
        : server.defaultEmbeddingDimension,
    defaultVisionProvider: visionProvider,
    defaultVisionModel: visionModel,
    extractionLanguage,
  };
}

/** Pick Default Workspace for language inheritance. */
export function pickDefaultWorkspaceLanguage(
  workspaces: Array<{
    slug?: string | null;
    name?: string | null;
    extraction_language?: string | null;
  }>,
): string | null {
  const auto =
    workspaces.find((w) => w.slug === 'default') ??
    workspaces.find((w) => w.name === 'Default Workspace') ??
    workspaces[0];
  const lang = auto?.extraction_language;
  if (typeof lang === 'string' && lang.trim().length > 0) return lang.trim();
  return null;
}

/**
 * LAW-101-13 — picker inherit chip must name the resolved provider/model.
 * Prefix is tenant vs server so it matches ServerDefaultsCard, not fleet catalog.
 */
export function formatInheritModelLabel(args: {
  source: DefaultsSource;
  provider?: string;
  model?: string;
  tenantPrefix?: string;
  serverPrefix?: string;
}): string {
  const prefix =
    args.source === 'tenant'
      ? (args.tenantPrefix ?? 'Organization default')
      : (args.serverPrefix ?? 'Server default');
  if (args.provider && args.model) {
    return `${prefix} (${args.provider}/${args.model})`;
  }
  return prefix;
}
