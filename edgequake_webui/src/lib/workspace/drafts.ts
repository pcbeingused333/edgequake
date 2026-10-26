import type { PdfParserBackendChoice } from "@/components/settings/pdf-parser-backend-field";
import type { EmbeddingSelection } from "@/components/workspace/embedding-model-selector";
import type { LLMSelection } from "@/components/workspace/llm-model-selector";
import type { Workspace } from "@/types";
import { savedVisionRole } from "@/lib/workspace/vision-choice";

export interface WorkspaceSelectionOpts {
  /**
   * When true, only return a pick if GET provenance is a workspace override.
   * Painted tenant/env values on `llm_model` must not look like overrides (LAW-101-5).
   */
  overridesOnly?: boolean;
}

/** Honest workspace-override signal (GET paints inherited llm/embedding onto struct fields). */
export function isWorkspaceResolutionSource(
  source?: string | null,
): boolean {
  return (source ?? "").trim().toLowerCase() === "workspace";
}

/** Mock must never be selected/saved in the application UI. */
function isMockProvider(provider?: string | null): boolean {
  const id = provider?.trim().toLowerCase() ?? "";
  return id === "mock" || id === "mock-imagegen";
}

/**
 * Infer a real provider from a model name when the workspace still has mock leftovers.
 * Mirrors server `Workspace::detect_provider_from_model` heuristics.
 */
function healMockProvider(provider: string, model: string): string {
  if (!isMockProvider(provider)) {
    return provider;
  }
  if (model.startsWith("text-embedding") || model.startsWith("ada")) {
    return "openai";
  }
  if (model.includes(":")) {
    return "ollama";
  }
  if (
    model.startsWith("mistral") ||
    model.startsWith("magistral") ||
    model.startsWith("codestral") ||
    model.startsWith("pixtral")
  ) {
    return "mistral";
  }
  if (model.startsWith("gpt-") || model.startsWith("o1") || model.startsWith("o3")) {
    return "openai";
  }
  return "ollama";
}

export function getWorkspaceLlmSelection(
  workspace?: Workspace | null,
  opts: WorkspaceSelectionOpts = {},
): LLMSelection | undefined {
  if (!workspace?.llm_provider || !workspace.llm_model) {
    return undefined;
  }
  if (opts.overridesOnly && !isWorkspaceResolutionSource(workspace.llm_resolution_source)) {
    return undefined;
  }

  const provider = healMockProvider(workspace.llm_provider, workspace.llm_model);
  return {
    model: workspace.llm_model,
    provider,
    fullId: `${provider}/${workspace.llm_model}`,
  };
}

export function getWorkspaceEmbeddingSelection(
  workspace?: Workspace | null,
  opts: WorkspaceSelectionOpts = {},
): EmbeddingSelection | undefined {
  if (!workspace?.embedding_provider || !workspace.embedding_model) {
    return undefined;
  }
  if (
    opts.overridesOnly &&
    !isWorkspaceResolutionSource(workspace.embedding_resolution_source)
  ) {
    return undefined;
  }

  const provider = healMockProvider(
    workspace.embedding_provider,
    workspace.embedding_model,
  );
  return {
    model: workspace.embedding_model,
    provider,
    dimension: workspace.embedding_dimension ?? 768,
  };
}

/** The saved vision choice: `llm_roles.vlm` (canonical), then the legacy vision fields. */
export function getWorkspaceVisionSelection(
  workspace?: Workspace | null,
): LLMSelection | undefined {
  const role = savedVisionRole(workspace);
  const rawProvider = role?.provider ?? workspace?.vision_llm_provider;
  const model = role?.model ?? workspace?.vision_llm_model;
  if (!rawProvider || !model) {
    return undefined;
  }

  const provider = healMockProvider(rawProvider, model);
  return {
    model,
    provider,
    fullId: `${provider}/${model}`,
  };
}

export function getWorkspacePdfParserBackend(
  workspace?: Workspace | null,
): PdfParserBackendChoice {
  return workspace?.pdf_parser_backend ?? "none";
}
