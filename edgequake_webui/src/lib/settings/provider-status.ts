/** SPEC-179 — pure view rules for the Deployment provider cards. */

import type { DeploymentProvider, OfferModel } from "@/lib/api/edgequake/provider-offer";

export type ProbeTone = "ok" | "failed" | "unchecked";

export function probeTone(probe: string | null | undefined): ProbeTone {
  if (probe === "ok") return "ok";
  if (probe === "failed") return "failed";
  return "unchecked";
}

export type Capability = "chat" | "vision" | "embedding";

/** The capability chips for one model. Chat is shown unless the model only embeds. */
export function capabilitiesOf(model: Pick<OfferModel, "usages">): Capability[] {
  const out: Capability[] = [];
  const usages = model.usages ?? [];
  if (usages.some((usage) => usage !== "embedding" && usage !== "vlm")) out.push("chat");
  if (usages.includes("vlm")) out.push("vision");
  if (usages.includes("embedding")) out.push("embedding");
  return out;
}

export type OfferState = "offered" | "hidden" | "not_offered";

export function offerState(row: Pick<DeploymentProvider, "withdrawn" | "offered">): OfferState {
  if (row.withdrawn) return "hidden";
  return row.offered ? "offered" : "not_offered";
}

/** "a1b2" stays readable when a model id is long. */
export function shortModelId(id: string, max = 34): string {
  return id.length <= max ? id : `${id.slice(0, max - 1)}…`;
}

export interface RefreshSummary {
  tone: "never" | "ok" | "error";
  added: number;
  removed: number;
}

export function refreshSummary(row: Pick<DeploymentProvider, "refresh">): RefreshSummary {
  const refresh = row.refresh;
  if (!refresh) return { tone: "never", added: 0, removed: 0 };
  return {
    tone: refresh.status === "ok" ? "ok" : "error",
    added: refresh.added,
    removed: refresh.removed,
  };
}

/** A model draft entry. `dimension` only matters for embedding models. */
export interface ModelDraft {
  id: string;
  kind: "chat" | "embedding";
  vision: boolean;
  dimension?: number;
}

export function draftFromOffer(models: OfferModel[]): ModelDraft[] {
  return models.map((model) => {
    const caps = capabilitiesOf(model);
    const embedding = caps.includes("embedding") && !caps.includes("chat");
    return {
      id: model.id,
      kind: embedding ? "embedding" : "chat",
      vision: caps.includes("vision"),
      dimension: embedding ? model.embedding_dimension : undefined,
    };
  });
}

export interface ConnectionModelPayload {
  id: string;
  supports_chat: boolean;
  supports_vision: boolean;
  supports_embedding: boolean;
  embedding_dimension?: number;
}

export function payloadFromDraft(draft: ModelDraft[]): ConnectionModelPayload[] {
  return draft.map((entry) =>
    entry.kind === "embedding"
      ? {
          id: entry.id,
          supports_chat: false,
          supports_vision: false,
          supports_embedding: true,
          embedding_dimension: entry.dimension,
        }
      : {
          id: entry.id,
          supports_chat: true,
          supports_vision: entry.vision,
          supports_embedding: false,
        },
  );
}

/** Ids of embedding entries that cannot be saved yet. */
export function embeddingsMissingDimension(draft: ModelDraft[]): string[] {
  return draft
    .filter((entry) => entry.kind === "embedding" && !(entry.dimension && entry.dimension > 0))
    .map((entry) => entry.id);
}

export function sameDraft(a: ModelDraft[], b: ModelDraft[]): boolean {
  return JSON.stringify(payloadFromDraft(a)) === JSON.stringify(payloadFromDraft(b));
}
