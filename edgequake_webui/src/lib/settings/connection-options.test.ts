import { describe, expect, it } from "vitest";
import { DEFAULT_CONNECTION_ID, selectableConnections } from "./connection-options";

describe("selectableConnections", () => {
  it("drops env rows that share the nil id and keeps one copy of a real id", () => {
    const rows = selectableConnections([
      { id: DEFAULT_CONNECTION_ID, slug: "ollama", source: "env" },
      { id: DEFAULT_CONNECTION_ID, slug: "openai", source: "env" },
      { id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", slug: "local", source: "db" },
      { id: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa", slug: "local-again", source: "db" },
    ]);
    expect(rows.map((row) => row.slug)).toEqual(["local"]);
  });
});
