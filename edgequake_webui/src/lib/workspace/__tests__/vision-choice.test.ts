import { describe, expect, it } from "vitest";
import { visionChoicePatch, savedVisionRole } from "@/lib/workspace/vision-choice";
import type { Workspace } from "@/types";

const roles = {
  extract: { provider: "openai", model: "gpt-5-nano" },
  vlm: { provider: "mistral", model: "pixtral-large-latest" },
} as unknown as Workspace["llm_roles"];

describe("visionChoicePatch", () => {
  it("writes the vlm role and clears the legacy vision fields", () => {
    const patch = visionChoicePatch(roles, { provider: "openai", model: "gpt-4o" });
    expect(patch.vision_llm_provider).toBe("");
    expect(patch.vision_llm_model).toBe("");
    expect(patch.llm_roles.vlm).toEqual({ provider: "openai", model: "gpt-4o" });
  });

  it("keeps the other saved roles intact", () => {
    const patch = visionChoicePatch(roles, { provider: "openai", model: "gpt-4o" });
    expect(patch.llm_roles.extract).toEqual({ provider: "openai", model: "gpt-5-nano" });
  });

  it("removes the vlm role when the choice is cleared (inherit)", () => {
    const patch = visionChoicePatch(roles, undefined);
    expect(patch.llm_roles.vlm).toBeUndefined();
    expect(patch.llm_roles.extract).toBeDefined();
  });

  it("carries a connection id on the vlm role when one is given", () => {
    const patch = visionChoicePatch(undefined, {
      provider: "openai",
      model: "gpt-4o",
      connection_id: "conn-1",
    });
    expect(patch.llm_roles.vlm).toEqual({
      provider: "openai",
      model: "gpt-4o",
      connection_id: "conn-1",
    });
  });
});

describe("savedVisionRole", () => {
  it("reads the canonical vlm role", () => {
    expect(savedVisionRole({ llm_roles: roles })).toEqual({
      provider: "mistral",
      model: "pixtral-large-latest",
    });
  });

  it("returns undefined when no vlm role is saved", () => {
    expect(savedVisionRole({ llm_roles: { extract: { provider: "x", model: "y" } } } as unknown as Workspace)).toBeUndefined();
    expect(savedVisionRole(null)).toBeUndefined();
  });
});
