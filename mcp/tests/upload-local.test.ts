import path from "node:path";
import { describe, expect, it } from "vitest";

import { resolveInsideRoot } from "../src/upload-local.js";

describe("resolveInsideRoot", () => {
  it("accepts a file inside the root", () => {
    const root = path.resolve("/tmp/eq-upload-root");
    expect(resolveInsideRoot(root, "notes/a.txt")).toBe(path.join(root, "notes/a.txt"));
  });

  it("rejects a path that escapes the root", () => {
    const root = path.resolve("/tmp/eq-upload-root");
    expect(() => resolveInsideRoot(root, "../secret.txt")).toThrow(/escapes/);
  });
});
