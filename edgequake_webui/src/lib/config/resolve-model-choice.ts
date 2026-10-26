/**
 * SPEC-123 model resolution mirror (LLM / embedding / vision LLM).
 *
 * Priority: Request/upload > Workspace > Tenant > Env > Default.
 * There is no separate “vision embedding” — vision is VLM; embedding is text vectors.
 *
 * @see edgequake-core `model_resolution.rs`
 */

export type ModelResolutionSource =
  | "request"
  | "workspace"
  | "tenant"
  | "env"
  | "default";

export interface ResolvedProviderModel {
  provider: string;
  model: string;
  source: ModelResolutionSource;
}

export interface ResolvedEmbedding {
  provider: string;
  model: string;
  dimension: number;
  source: ModelResolutionSource;
}

function nonEmpty(value?: string | null): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function firstLayer(
  layers: Array<{
    provider?: string | null;
    model?: string | null;
    source: ModelResolutionSource;
  }>,
): { provider: string; model: string; source: ModelResolutionSource } | null {
  for (const layer of layers) {
    const p = nonEmpty(layer.provider);
    const m = nonEmpty(layer.model);
    if (!p && !m) continue;
    return { provider: p ?? "", model: m ?? "", source: layer.source };
  }
  return null;
}

function fillGaps(
  won: { provider: string; model: string; source: ModelResolutionSource },
  layers: Array<{
    provider?: string | null;
    model?: string | null;
    source: ModelResolutionSource;
  }>,
  defaults: { provider: string; model: string },
): ResolvedProviderModel {
  let { provider, model, source } = won;
  for (const layer of layers) {
    if (!provider) {
      const p = nonEmpty(layer.provider);
      if (p) provider = p;
    }
    if (!model) {
      const m = nonEmpty(layer.model);
      if (m) model = m;
    }
    if (provider && model) break;
  }
  if (!provider) provider = defaults.provider;
  if (!model) model = defaults.model;
  return { provider, model, source };
}

export function resolveLlmChoice(ctx: {
  requestProvider?: string | null;
  requestModel?: string | null;
  workspaceProvider?: string | null;
  workspaceModel?: string | null;
  tenantProvider?: string | null;
  tenantModel?: string | null;
  envProvider?: string;
  envModel?: string;
}): ResolvedProviderModel {
  const envProvider = ctx.envProvider ?? "ollama";
  const envModel = ctx.envModel ?? "gemma4:latest";
  const layers = [
    {
      provider: ctx.requestProvider,
      model: ctx.requestModel,
      source: "request" as const,
    },
    {
      provider: ctx.workspaceProvider,
      model: ctx.workspaceModel,
      source: "workspace" as const,
    },
    {
      provider: ctx.tenantProvider,
      model: ctx.tenantModel,
      source: "tenant" as const,
    },
    { provider: envProvider, model: envModel, source: "env" as const },
  ];
  const won = firstLayer(layers) ?? {
    provider: envProvider,
    model: envModel,
    source: "env" as const,
  };
  return fillGaps(won, layers, { provider: envProvider, model: envModel });
}

export function resolveEmbeddingChoice(ctx: {
  requestProvider?: string | null;
  requestModel?: string | null;
  requestDimension?: number | null;
  workspaceProvider?: string | null;
  workspaceModel?: string | null;
  workspaceDimension?: number | null;
  tenantProvider?: string | null;
  tenantModel?: string | null;
  tenantDimension?: number | null;
  envProvider?: string;
  envModel?: string;
  envDimension?: number;
}): ResolvedEmbedding {
  const pair = resolveLlmChoice({
    requestProvider: ctx.requestProvider,
    requestModel: ctx.requestModel,
    workspaceProvider: ctx.workspaceProvider,
    workspaceModel: ctx.workspaceModel,
    tenantProvider: ctx.tenantProvider,
    tenantModel: ctx.tenantModel,
    envProvider: ctx.envProvider ?? "openai",
    envModel: ctx.envModel ?? "text-embedding-3-small",
  });
  const dimension =
    (ctx.requestDimension && ctx.requestDimension > 0
      ? ctx.requestDimension
      : undefined) ??
    (ctx.workspaceDimension && ctx.workspaceDimension > 0
      ? ctx.workspaceDimension
      : undefined) ??
    (ctx.tenantDimension && ctx.tenantDimension > 0
      ? ctx.tenantDimension
      : undefined) ??
    ctx.envDimension ??
    1536;
  return { ...pair, dimension };
}

/** Vision LLM (VLM) — not an embedding model. */
export function resolveVisionLlmChoice(ctx: {
  uploadProvider?: string | null;
  uploadModel?: string | null;
  workspaceVisionProvider?: string | null;
  workspaceVisionModel?: string | null;
  workspaceLlmProvider?: string | null;
  workspaceLlmModel?: string | null;
  tenantVisionProvider?: string | null;
  tenantVisionModel?: string | null;
  envProvider?: string;
  envModel?: string;
}): ResolvedProviderModel {
  const envProvider = ctx.envProvider ?? "ollama";
  const envModel = ctx.envModel ?? "gemma4:latest";
  const layers = [
    {
      provider: ctx.uploadProvider,
      model: ctx.uploadModel,
      source: "request" as const,
    },
    {
      provider: ctx.workspaceVisionProvider,
      model: ctx.workspaceVisionModel,
      source: "workspace" as const,
    },
    {
      provider: ctx.tenantVisionProvider,
      model: ctx.tenantVisionModel,
      source: "tenant" as const,
    },
    {
      provider: ctx.workspaceLlmProvider,
      model: ctx.workspaceLlmModel,
      source: "workspace" as const,
    },
    { provider: envProvider, model: envModel, source: "env" as const },
  ];
  const won = firstLayer(layers) ?? {
    provider: envProvider,
    model: envModel,
    source: "env" as const,
  };
  return fillGaps(won, layers, { provider: envProvider, model: envModel });
}

/** Prefer API-resolved vision fields (SPEC-123 honesty) over local mirror. */
export function effectiveVisionFromWorkspace(workspace: {
  vision_llm_provider?: string | null;
  vision_llm_model?: string | null;
  resolved_vision_llm_provider?: string | null;
  resolved_vision_llm_model?: string | null;
  vision_llm_resolution_source?: string | null;
  llm_provider?: string | null;
  llm_model?: string | null;
  llm_roles?: Record<string, { provider?: string; model?: string } | undefined> | null;
}): ResolvedProviderModel {
  if (
    workspace.resolved_vision_llm_provider &&
    workspace.resolved_vision_llm_model
  ) {
    return {
      provider: workspace.resolved_vision_llm_provider,
      model: workspace.resolved_vision_llm_model,
      source:
        (workspace.vision_llm_resolution_source as ModelResolutionSource) ||
        "env",
    };
  }
  // `llm_roles.vlm` is canonical (SPEC-163); the legacy vision fields only fill in behind it.
  const role = workspace.llm_roles?.vlm;
  return resolveVisionLlmChoice({
    workspaceVisionProvider: role?.provider || workspace.vision_llm_provider,
    workspaceVisionModel: role?.model || workspace.vision_llm_model,
    workspaceLlmProvider: workspace.llm_provider,
    workspaceLlmModel: workspace.llm_model,
  });
}

/** Prefer API-resolved LLM fields. */
export function effectiveLlmFromWorkspace(workspace: {
  llm_provider?: string | null;
  llm_model?: string | null;
  resolved_llm_provider?: string | null;
  resolved_llm_model?: string | null;
  llm_resolution_source?: string | null;
}): ResolvedProviderModel {
  if (workspace.resolved_llm_provider && workspace.resolved_llm_model) {
    return {
      provider: workspace.resolved_llm_provider,
      model: workspace.resolved_llm_model,
      source: (workspace.llm_resolution_source as ModelResolutionSource) || "env",
    };
  }
  return resolveLlmChoice({
    workspaceProvider: workspace.llm_provider,
    workspaceModel: workspace.llm_model,
  });
}

/** Prefer API-resolved embedding fields. */
export function effectiveEmbeddingFromWorkspace(workspace: {
  embedding_provider?: string | null;
  embedding_model?: string | null;
  embedding_dimension?: number | null;
  resolved_embedding_provider?: string | null;
  resolved_embedding_model?: string | null;
  resolved_embedding_dimension?: number | null;
  embedding_resolution_source?: string | null;
}): ResolvedEmbedding {
  if (
    workspace.resolved_embedding_provider &&
    workspace.resolved_embedding_model
  ) {
    return {
      provider: workspace.resolved_embedding_provider,
      model: workspace.resolved_embedding_model,
      dimension: workspace.resolved_embedding_dimension ?? 1536,
      source:
        (workspace.embedding_resolution_source as ModelResolutionSource) ||
        "env",
    };
  }
  return resolveEmbeddingChoice({
    workspaceProvider: workspace.embedding_provider,
    workspaceModel: workspace.embedding_model,
    workspaceDimension: workspace.embedding_dimension,
  });
}
