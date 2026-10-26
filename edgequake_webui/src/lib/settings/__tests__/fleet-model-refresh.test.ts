import { describe, expect, it } from "vitest";
import {
  showFleetModelRefresh,
  showProviderModelRefresh,
} from "../fleet-model-refresh";

describe("showProviderModelRefresh", () => {
  it("refreshes a refreshable env slug when the probe is ok or unset", () => {
    expect(showProviderModelRefresh({ refreshable: true, origin: "env", probe: "ok" })).toBe(true);
    expect(showProviderModelRefresh({ refreshable: true, origin: "env", probe: "" })).toBe(true);
    expect(showProviderModelRefresh({ refreshable: true, origin: "env" })).toBe(true);
  });

  it("refreshes a refreshable fleet slug only after a successful probe", () => {
    expect(showProviderModelRefresh({ refreshable: true, origin: "fleet", probe: "ok" })).toBe(true);
    expect(showProviderModelRefresh({ refreshable: true, origin: "fleet", probe: "failed" })).toBe(false);
  });

  it("hides refresh when the server does not mark the slug refreshable", () => {
    expect(showProviderModelRefresh({ refreshable: false, origin: "env", probe: "ok" })).toBe(false);
    expect(showProviderModelRefresh({ refreshable: false, origin: "fleet", probe: "ok" })).toBe(false);
  });
});

describe("showFleetModelRefresh", () => {
  it("shows refresh for every probe-ok refreshable slug", () => {
    expect(showFleetModelRefresh(true, true)).toBe(true);
  });

  it("hides refresh when the server does not mark the slug refreshable", () => {
    expect(showFleetModelRefresh(false, true)).toBe(false);
  });

  it("hides refresh until the connection probe succeeds", () => {
    expect(showFleetModelRefresh(true, false)).toBe(false);
  });
});
