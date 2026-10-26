import { describe, expect, it } from "vitest";
import {
  capabilitiesOf,
  draftFromOffer,
  embeddingsMissingDimension,
  offerState,
  payloadFromDraft,
  probeTone,
  refreshSummary,
  sameDraft,
  shortModelId,
} from "../provider-status";

describe("provider status", () => {
  it("maps probe and offer states", () => {
    expect(probeTone("ok")).toBe("ok");
    expect(probeTone("failed")).toBe("failed");
    expect(probeTone(null)).toBe("unchecked");
    expect(offerState({ withdrawn: true, offered: true })).toBe("hidden");
    expect(offerState({ withdrawn: false, offered: true })).toBe("offered");
    expect(offerState({ withdrawn: false, offered: false })).toBe("not_offered");
  });

  it("derives capability chips from usages", () => {
    expect(capabilitiesOf({ usages: ["query", "extract"] })).toEqual(["chat"]);
    expect(capabilitiesOf({ usages: ["query", "vlm"] })).toEqual(["chat", "vision"]);
    expect(capabilitiesOf({ usages: ["embedding"] })).toEqual(["embedding"]);
    expect(capabilitiesOf({ usages: [] })).toEqual([]);
  });

  it("summarises the last refresh", () => {
    expect(refreshSummary({})).toEqual({ tone: "never", added: 0, removed: 0 });
    expect(
      refreshSummary({ refresh: { status: "ok", error: null, added: 2, removed: 1, refreshed_at: "2026-01-01T00:00:00Z" } }),
    ).toEqual({ tone: "ok", added: 2, removed: 1 });
    expect(
      refreshSummary({ refresh: { status: "error", error: "boom", added: 0, removed: 0, refreshed_at: "2026-01-01T00:00:00Z" } }).tone,
    ).toBe("error");
  });

  it("truncates long ids", () => {
    expect(shortModelId("abc", 10)).toBe("abc");
    expect(shortModelId("a".repeat(50), 10)).toHaveLength(10);
  });
});

describe("model draft payload", () => {
  const offer = [
    { id: "gpt-x", usages: ["query", "vlm"] },
    { id: "embed-y", usages: ["embedding"], embedding_dimension: 768 },
  ];

  it("round-trips an offer into the exact connection body", () => {
    expect(payloadFromDraft(draftFromOffer(offer))).toEqual([
      { id: "gpt-x", supports_chat: true, supports_vision: true, supports_embedding: false },
      { id: "embed-y", supports_chat: false, supports_vision: false, supports_embedding: true, embedding_dimension: 768 },
    ]);
  });

  it("flags an embedding without a dimension and detects edits", () => {
    const draft = draftFromOffer(offer);
    expect(embeddingsMissingDimension(draft)).toEqual([]);
    const edited = draft.map((entry) => (entry.kind === "embedding" ? { ...entry, dimension: undefined } : entry));
    expect(embeddingsMissingDimension(edited)).toEqual(["embed-y"]);
    expect(sameDraft(draft, draftFromOffer(offer))).toBe(true);
    expect(sameDraft(draft, edited)).toBe(false);
  });
});
