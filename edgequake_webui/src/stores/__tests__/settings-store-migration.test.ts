import { describe, expect, it } from "vitest";
import { adoptLegacyQueryModel, dropRetiredSettingsKeys, visibleChoiceModels } from "../use-settings-store";

describe("dropRetiredSettingsKeys (EC-175-23, store version 2)", () => {
  it("drops the persisted theme, which next-themes owns", () => {
    const out = dropRetiredSettingsKeys({ theme: "dark", language: "fr" });
    expect(out).not.toHaveProperty("theme");
    expect(out.language).toBe("fr");
  });

  it("drops the gleaning block that no reader used", () => {
    const out = dropRetiredSettingsKeys({ ingestionSettings: { enableGleaning: true }, language: "en" });
    expect(out).not.toHaveProperty("ingestionSettings");
  });

  it("drops rerank keys from query settings and keeps the rest", () => {
    const out = dropRetiredSettingsKeys({
      querySettings: { mode: "local", enableRerank: true, rerankTopK: 10, stream: false },
    });
    expect(out.querySettings).toEqual({ mode: "local", stream: false });
  });

  it("tolerates a v1 blob with no query settings", () => {
    expect(dropRetiredSettingsKeys({ theme: "light" })).toEqual({ querySettings: {} });
  });

  it("copies a legacy pair onto the open workspace once (EC-178-27)", () => {
    const first = adoptLegacyQueryModel({
      map: {},
      legacy: { provider: "openai", model: "gpt-5-nano" },
      workspaceId: "ws-a",
      adopted: false,
    });
    expect(first.map["ws-a"]).toEqual({ provider: "openai", model: "gpt-5-nano" });
    expect(first.adopted).toBe(true);
    const second = adoptLegacyQueryModel({
      map: first.map,
      legacy: { provider: "openai", model: "gpt-5-nano" },
      workspaceId: "ws-b",
      adopted: first.adopted,
    });
    expect(second.map["ws-b"]).toBeUndefined();
    expect(second.wrote).toBe(false);
  });

  it("drops the mock provider from a choice list (EC-178-22)", () => {
    expect(visibleChoiceModels([
      { provider: "mock", model: "mock-smart" },
      { provider: "openai", model: "gpt-5-nano" },
    ])).toEqual([{ provider: "openai", model: "gpt-5-nano" }]);
  });

  it("does not mutate its input", () => {
    const input = { theme: "dark", querySettings: { enableRerank: true } };
    dropRetiredSettingsKeys(input);
    expect(input.theme).toBe("dark");
    expect(input.querySettings).toEqual({ enableRerank: true });
  });
});
