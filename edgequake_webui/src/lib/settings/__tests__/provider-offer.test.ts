import { describe, expect, it } from "vitest";
import { draftFromProviders, lead, sameDraft, slugsForSave, toggleSlug } from "../provider-offer";

describe("provider offer draft", () => {
  it("omits ticks of a provider that is off", () => {
    const draft = toggleSlug(
      { mode: "unconstrained", slugs: ["openai", "acme"] },
      "acme",
      ["openai", "acme"],
    );
    expect(draft.mode).toBe("constrained");
    expect(slugsForSave(draft)).toEqual(["openai"]);
  });

  it("an empty constrained draft offers nothing", () => {
    const draft = { mode: "constrained" as const, slugs: [] };
    expect(slugsForSave(draft)).toEqual([]);
    expect(sameDraft(draft, draftFromProviders("constrained", []))).toBe(true);
  });

  it("names the parent in the lead", () => {
    expect(lead("tenant", "unconstrained")).toContain("this deployment");
    expect(lead("workspace", "constrained")).toContain("this workspace");
  });
});
