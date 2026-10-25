import { describe, expect, it } from "vitest";
import { dropRetiredSettingsKeys } from "../use-settings-store";

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

  it("does not mutate its input", () => {
    const input = { theme: "dark", querySettings: { enableRerank: true } };
    dropRetiredSettingsKeys(input);
    expect(input.theme).toBe("dark");
    expect(input.querySettings).toEqual({ enableRerank: true });
  });
});
