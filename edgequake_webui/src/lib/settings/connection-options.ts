/** The synthetic id on every env-backed connection. It means "inherit", not a saved row. */
export const DEFAULT_CONNECTION_ID = "00000000-0000-0000-0000-000000000000";

export interface ConnectionChoice {
  id: string;
  slug?: string;
  source?: string;
}

/**
 * Connections a role may pin. Env rows share one nil id, so they are dropped here.
 * A repeated real id is kept once, so the option list never has two children with the same key.
 */
export function selectableConnections<T extends ConnectionChoice>(rows: readonly T[]): T[] {
  const seen = new Set<string>();
  const out: T[] = [];
  for (const row of rows) {
    if (!row.id || row.id === DEFAULT_CONNECTION_ID || seen.has(row.id)) continue;
    seen.add(row.id);
    out.push(row);
  }
  return out;
}
