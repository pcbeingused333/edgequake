import { describe, expect, it } from "vitest";
import type { LlmModelItem } from "@/lib/api/models";
import {
  connectionModelBody,
  showFleetModelRefresh,
  showProviderModelRefresh,
  toggleModelId,
} from "../fleet-model-refresh";

const gemma = {
  name: "gemma4:latest",
  capabilities: { supports_vision: true },
} as LlmModelItem;

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

describe("connectionModelBody", () => {
  it("keeps vision from the discovered card and chat for query eligibility", () => {
    expect(connectionModelBody(["gemma4:latest", "typed-id"], [gemma])).toEqual([
      {
        id: "gemma4:latest",
        supports_chat: true,
        supports_vision: true,
        supports_embedding: false,
      },
      {
        id: "typed-id",
        supports_chat: true,
        supports_vision: false,
        supports_embedding: false,
      },
    ]);
  });
});

describe("toggleModelId", () => {
  it("adds and removes without duplicates", () => {
    expect(toggleModelId(["a"], "b", true)).toEqual(["a", "b"]);
    expect(toggleModelId(["a", "b"], "b", true)).toEqual(["a", "b"]);
    expect(toggleModelId(["a", "b"], "a", false)).toEqual(["b"]);
  });
});
