import { describe, expect, it } from "vitest";
import type { EligibleModel, ResolvedUsage } from "@/lib/api/edgequake/model-catalog";
import { choiceSendBlocked, inheritIsPossible } from "../choice-possibility";

const gemma: EligibleModel = {
  provider: "ollama",
  provider_display_name: "Ollama",
  model: "gemma4:latest",
  display_name: "Gemma 4",
  supports_vision: false,
};

function usage(partial: Partial<ResolvedUsage>): ResolvedUsage {
  return {
    usage: "query",
    provider: "ollama",
    provider_display_name: "Ollama",
    model: "gemma4:latest",
    connection_id: null,
    reasoning_effort: null,
    source: "workspace_default",
    locked: false,
    eligible: true,
    broken_reason: "none",
    ...partial,
  };
}

describe("inheritIsPossible", () => {
  it("accepts an eligible workspace default that is on the list", () => {
    expect(inheritIsPossible(usage({}), [gemma])).toBe(true);
  });

  it("rejects a broken not_eligible default even if the label matches (EC-178 inherit)", () => {
    expect(
      inheritIsPossible(
        usage({
          provider: "mistral",
          model: "mistral-small-latest",
          eligible: false,
          broken_reason: "not_eligible",
        }),
        [gemma],
      ),
    ).toBe(false);
  });

  it("rejects a healthy-looking default that the eligible list omits", () => {
    expect(
      inheritIsPossible(usage({ provider: "mistral", model: "mistral-small-latest" }), [gemma]),
    ).toBe(false);
  });
});

describe("choiceSendBlocked", () => {
  it("blocks when the eligible set is empty", () => {
    expect(
      choiceSendBlocked({
        eligibleLoaded: true,
        eligibleEmpty: true,
        inheritOk: false,
      }),
    ).toBe(true);
  });

  it("blocks Inherit when the default cannot run and no override is set", () => {
    expect(
      choiceSendBlocked({
        eligibleLoaded: true,
        eligibleEmpty: false,
        inheritOk: false,
      }),
    ).toBe(true);
  });

  it("allows Send when Inherit is possible", () => {
    expect(
      choiceSendBlocked({
        eligibleLoaded: true,
        eligibleEmpty: false,
        inheritOk: true,
      }),
    ).toBe(false);
  });

  it("allows Send when the member picked an eligible override", () => {
    expect(
      choiceSendBlocked({
        eligibleLoaded: true,
        eligibleEmpty: false,
        inheritOk: false,
        overrideProvider: "ollama",
        overrideModel: "gemma4:latest",
      }),
    ).toBe(false);
  });
});
