/** SPEC-175 Connections: one-line facts about a saved connection. Pure, no React. */

export interface ConnectionSummaryInput {
  api_shape: string;
  base_url: string;
  source: string;
  key_configured: boolean;
  key_fingerprint?: string | null;
  last_test_ok?: boolean | null;
}

const SOURCE_LABELS: Readonly<Record<string, string>> = {
  user: "Saved by you",
  env: "From environment",
};

/** Human label for where a connection came from. Unknown sources pass through. */
export function connectionSourceLabel(source: string): string {
  const label = Object.hasOwn(SOURCE_LABELS, source) ? SOURCE_LABELS[source] : undefined;
  return label ?? source;
}

/**
 * The non-empty facts about a connection, in display order. Blank fields are
 * dropped here so the caller can join with " · " without doubled separators.
 */
export function connectionSummaryParts(connection: ConnectionSummaryInput): string[] {
  const parts: Array<string | null | undefined | false> = [
    connection.api_shape,
    connection.base_url,
    connectionSourceLabel(connection.source),
    connection.key_configured && `key ${connection.key_fingerprint ?? "set"}`,
    connection.last_test_ok === false && "last test failed",
  ];
  return parts.filter((part): part is string => typeof part === "string" && part.trim() !== "");
}

/** The summary line shown under a connection's name. */
export function connectionSummary(connection: ConnectionSummaryInput): string {
  return connectionSummaryParts(connection).join(" · ");
}
