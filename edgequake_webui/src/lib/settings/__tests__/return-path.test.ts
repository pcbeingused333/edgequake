import { describe, expect, it } from "vitest";
import { isSettingsPath, rememberedAppPath, rememberAppPath } from "../return-path";

describe("isSettingsPath", () => {
  it("is true for scope routes and workspace deeplinks", () => {
    expect(isSettingsPath("/settings")).toBe(true);
    expect(isSettingsPath("/settings/you/appearance")).toBe(true);
    expect(isSettingsPath("/w/acme/settings")).toBe(true);
    expect(isSettingsPath("/w/acme/settings/access")).toBe(true);
  });

  it("is false for app routes, including ones that merely start with settings", () => {
    expect(isSettingsPath("/documents")).toBe(false);
    expect(isSettingsPath("/settingsx")).toBe(false);
    expect(isSettingsPath("/w/acme/documents")).toBe(false);
  });
});

describe("return path storage is safe without a window", () => {
  it("does nothing server-side and returns null", () => {
    rememberAppPath("/documents");
    expect(rememberedAppPath()).toBeNull();
  });
});
