import { describe, expect, it } from "vitest";
import {
  connectionSourceLabel,
  connectionSummary,
  connectionSummaryParts,
} from "./connection-summary";

const base = {
  api_shape: "ollama",
  base_url: "http://localhost:11434",
  source: "user",
  key_configured: false,
  key_fingerprint: null,
  last_test_ok: null,
};

describe("connectionSourceLabel", () => {
  it("names the known sources in sentence case", () => {
    expect(connectionSourceLabel("user")).toBe("Saved by you");
    expect(connectionSourceLabel("env")).toBe("From environment");
  });

  it("passes unknown sources through unchanged", () => {
    expect(connectionSourceLabel("workspace")).toBe("workspace");
  });
});

describe("connectionSummary", () => {
  it("joins shape, base url and source", () => {
    expect(connectionSummary(base)).toBe(
      "ollama · http://localhost:11434 · Saved by you",
    );
  });

  it("never leaves a doubled separator when fields are blank", () => {
    const summary = connectionSummary({ ...base, api_shape: "", base_url: "  " });
    expect(summary).toBe("Saved by you");
    expect(summary).not.toContain("··");
    expect(summary).not.toMatch(/^\s*·/);
  });

  it("adds the key fingerprint only when a key is configured", () => {
    expect(connectionSummary({ ...base, key_configured: true, key_fingerprint: "sk-…a1" })).toContain(
      "key sk-…a1",
    );
    expect(connectionSummary({ ...base, key_configured: true, key_fingerprint: null })).toContain(
      "key set",
    );
    expect(connectionSummary(base)).not.toContain("key");
  });

  it("flags a failed last test", () => {
    expect(connectionSummary({ ...base, last_test_ok: false })).toMatch(/last test failed$/);
    expect(connectionSummary({ ...base, last_test_ok: true })).not.toContain("failed");
  });
});

describe("connectionSummaryParts", () => {
  it("returns no parts for an empty connection", () => {
    expect(
      connectionSummaryParts({
        api_shape: "",
        base_url: "",
        source: "",
        key_configured: false,
      }),
    ).toEqual([]);
  });
});
