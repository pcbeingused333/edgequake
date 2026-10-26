/**
 * The one write path for the workspace vision (VLM) model (SPEC-040 / SPEC-163).
 *
 * `llm_roles.vlm` is the canonical choice. The legacy `vision_llm_*` fields are cleared on every
 * save, so no stale value can resolve after a new choice (or an inherit) is saved.
 */
import {
  buildLlmRoles,
  draftFromWorkspaceRoles,
  type LlmRoleValue,
} from "@/lib/settings/model-usage";
import type { Workspace } from "@/types/workspace";

export interface VisionChoice {
  provider: string;
  model: string;
  connection_id?: string;
}

export interface VisionChoicePatch {
  vision_llm_provider: string;
  vision_llm_model: string;
  llm_roles: Record<string, LlmRoleValue>;
}

/**
 * Patch that sets (or, with `undefined`, clears) the workspace vision choice. Other roles keep
 * their saved values, so saving vision never changes extraction or query roles.
 */
export function visionChoicePatch(
  savedRoles: Workspace["llm_roles"] | undefined,
  choice: VisionChoice | undefined,
): VisionChoicePatch {
  const draft = draftFromWorkspaceRoles(savedRoles);
  draft.vlm = {
    provider: choice?.provider ?? "",
    model: choice?.model ?? "",
    connection_id: choice?.connection_id ?? "",
  };
  return {
    vision_llm_provider: "",
    vision_llm_model: "",
    llm_roles: buildLlmRoles(draft),
  };
}

/** The saved `llm_roles.vlm` choice, if one is set. */
export function savedVisionRole(
  workspace: Pick<Workspace, "llm_roles"> | null | undefined,
): { provider?: string; model?: string } | undefined {
  const vlm = workspace?.llm_roles?.vlm;
  if (!vlm?.provider && !vlm?.model) return undefined;
  return { provider: vlm.provider || undefined, model: vlm.model || undefined };
}
