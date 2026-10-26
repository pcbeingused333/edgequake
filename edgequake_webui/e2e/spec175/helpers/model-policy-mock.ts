/**
 * SPEC-175 W4/W5 — hermetic model-policy server for the typed editors.
 *
 * Holds the policy in memory and answers reads and full-replace writes the way the server does:
 * eligibility = registered → supports usage → probe not failed → tenant ceiling → workspace
 * narrowing. Every PUT is recorded so a test can assert on exactly what was sent.
 */
import type { Page, Route } from "@playwright/test";
import { SPEC155_TENANT, SPEC155_WORKSPACE } from "../../spec155/helpers/mock-api";

type UsageKey = "extract" | "query" | "summary" | "vlm" | "keyword" | "embedding" | "decision";
type Health = "healthy" | "probe_failed" | "unknown";
interface PolicyRow { provider: string; model: string; usage: UsageKey; enabled: boolean }
interface RoleValue { provider?: string; model?: string; connection_id?: string }

/** Registered deployment models. `qwen3` is registered but its last probe failed. */
export const SPEC175_CATALOG = [
  { provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", display_name: "GPT-5 nano", health: "healthy" as Health, usages: ["extract", "query", "summary", "keyword"] as UsageKey[], supports_vision: false },
  { provider: "ollama", provider_display_name: "Ollama", model: "gemma3", display_name: "Gemma 3", health: "healthy" as Health, usages: ["extract", "query", "summary", "keyword", "vlm"] as UsageKey[], supports_vision: true },
  { provider: "ollama", provider_display_name: "Ollama", model: "qwen3", display_name: "Qwen 3", health: "probe_failed" as Health, usages: ["extract", "query"] as UsageKey[], supports_vision: false },
  { provider: "ollama", provider_display_name: "Ollama", model: "nomic-embed", display_name: "Nomic Embed", health: "unknown" as Health, usages: ["embedding"] as UsageKey[], supports_vision: false },
];

const USAGES: UsageKey[] = ["extract", "query", "summary", "vlm", "keyword", "embedding", "decision"];
const key = (provider: string, model: string) => `${provider}::${model}`;

export interface ModelPolicyScenario {
  /** Whether the caller may change this workspace's models and narrowing. */
  workspaceCanManage: boolean;
  /** Starting organization ceiling. Empty = unconstrained. */
  tenantRows?: PolicyRow[];
  /** Starting workspace narrowing. */
  workspaceRows?: PolicyRow[];
  /** Starting workspace `llm_roles`. */
  roles?: Partial<Record<UsageKey, RoleValue>>;
  /** Keep `keyword` set by the operator (locked). */
  lockKeyword?: boolean;
  /** Keep `query` set by the operator (locked). */
  lockQuery?: boolean;
  /** When false, every eligible model reports `supports_streaming: false`. */
  supportsStreaming?: boolean;
}

export interface ModelPolicyRecorder {
  tenantPuts: PolicyRow[][];
  workspacePuts: PolicyRow[][];
  rolePuts: Record<string, RoleValue>[];
}

function eligibleFor(
  usage: UsageKey,
  tenantRows: PolicyRow[],
  workspaceRows: PolicyRow[],
  supportsStreaming?: boolean,
) {
  const ceiling = tenantRows.filter((r) => r.enabled && r.usage === usage).map((r) => key(r.provider, r.model));
  const disabled = new Set(
    workspaceRows.filter((r) => !r.enabled && r.usage === usage).map((r) => key(r.provider, r.model)),
  );
  return SPEC175_CATALOG.filter(
    (m) =>
      m.usages.includes(usage) &&
      m.health !== "probe_failed" &&
      (ceiling.length === 0 || ceiling.includes(key(m.provider, m.model))) &&
      !disabled.has(key(m.provider, m.model)),
  ).map(({ health: _health, usages: _usages, ...m }) =>
    supportsStreaming === false ? { ...m, supports_streaming: false } : m,
  );
}

const json = (route: Route, body: unknown, status = 200) =>
  route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });

/** Installs the model-policy routes. Call after `prepareSpec168Page` so these win (LIFO). */
export async function mockModelPolicy(page: Page, scenario: ModelPolicyScenario): Promise<ModelPolicyRecorder> {
  const recorder: ModelPolicyRecorder = { tenantPuts: [], workspacePuts: [], rolePuts: [] };
  let tenantRows: PolicyRow[] = scenario.tenantRows ?? [];
  let workspaceRows: PolicyRow[] = scenario.workspaceRows ?? [];
  let roles: Partial<Record<UsageKey, RoleValue>> = scenario.roles ?? {
    extract: { provider: "ollama", model: "gemma3" },
    query: { provider: "openai", model: "gpt-5-nano" },
    vlm: { provider: "ollama", model: "qwen3" },
  };

  const workspaceView = () => ({ ...SPEC155_WORKSPACE, llm_roles: roles });

  const effectiveUsage = (usage: UsageKey) => {
    const eligible = eligibleFor(usage, tenantRows, workspaceRows, scenario.supportsStreaming);
    if (usage === "query" && scenario.lockQuery) {
      return { usage, provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", connection_id: null, reasoning_effort: null, source: "operator_lock", locked: true, eligible: true, broken_reason: "none" };
    }
    if (usage === "keyword" && scenario.lockKeyword) {
      return { usage, provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", connection_id: null, reasoning_effort: null, source: "operator_lock", locked: true, eligible: true, broken_reason: "none" };
    }
    const role = roles[usage];
    const provider = role?.provider ?? "ollama";
    const model = role?.model ?? "gemma3";
    const ok = eligible.some((m) => m.provider === provider && m.model === model);
    return {
      usage,
      provider,
      provider_display_name: provider === "openai" ? "OpenAI" : "Ollama",
      model,
      connection_id: role?.connection_id ?? null,
      reasoning_effort: null,
      source: role ? "workspace_role" : "tenant_default",
      locked: false,
      eligible: ok,
      broken_reason: ok ? "none" : "not_eligible",
    };
  };

  const policyMode = () => (tenantRows.length > 0 ? "constrained" : "unconstrained");

  // Workspace list and by-slug lookups carry the saved roles, so the editor starts from them.
  await page.route("**/api/v1/tenants/*/workspaces**", (route) => {
    if (route.request().url().includes("/by-slug/")) return json(route, workspaceView());
    return json(route, { items: [workspaceView()], total: 1, offset: 0, limit: 20 });
  });
  // Anchored so `/stats` and `/model-policy` under the same id are not swallowed here.
  const workspaceUrl = new RegExp(`/api/v1/workspaces/${SPEC155_WORKSPACE.id}(\\?.*)?$`);
  await page.route(workspaceUrl, async (route) => {
    if (route.request().method() !== "PUT") return json(route, workspaceView());
    const body = route.request().postDataJSON() as { llm_roles?: Record<string, RoleValue> };
    recorder.rolePuts.push(body.llm_roles ?? {});
    roles = (body.llm_roles ?? {}) as Partial<Record<UsageKey, RoleValue>>;
    return json(route, workspaceView());
  });

  await page.route("**/api/v1/config/effective**", (route) =>
    json(route, {
      workspace_id: SPEC155_WORKSPACE.id,
      policy_mode: policyMode(),
      usages: USAGES.map(effectiveUsage),
    }),
  );

  await page.route("**/api/v1/models/eligible**", (route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") as UsageKey;
    return json(route, {
      usage,
      policy_mode: tenantRows.some((r) => r.usage === usage) ? "constrained" : "unconstrained",
      models: eligibleFor(usage, tenantRows, workspaceRows, scenario.supportsStreaming),
    });
  });

  await page.route("**/api/v1/connections**", (route) =>
    json(route, [{ id: "conn-local-ollama", slug: "local-ollama", display_name: "Local Ollama", source: "user" }]),
  );

  await page.route("**/api/v1/tenants/*/model-catalog**", (route) =>
    json(route, {
      policy_mode: policyMode(),
      entries: SPEC175_CATALOG.map((m) => ({
        provider: m.provider,
        provider_display_name: m.provider_display_name,
        model: m.model,
        display_name: m.display_name,
        health: m.health,
        usages: m.usages,
      })),
    }),
  );

  await page.route("**/api/v1/tenants/*/model-policy**", async (route) => {
    const tenantPolicy = (rows: PolicyRow[]) => ({
      tenant_id: SPEC155_TENANT.id,
      constrained_usages: [...new Set(rows.filter((r) => r.enabled).map((r) => r.usage))],
      rows,
    });
    if (route.request().method() !== "PUT") return json(route, tenantPolicy(tenantRows));
    const body = route.request().postDataJSON() as { rows: PolicyRow[] };
    recorder.tenantPuts.push(body.rows);
    tenantRows = body.rows;
    return json(route, tenantPolicy(tenantRows));
  });

  await page.route("**/api/v1/workspaces/*/model-policy**", async (route) => {
    const workspacePolicy = (rows: PolicyRow[]) => ({
      workspace_id: SPEC155_WORKSPACE.id,
      can_manage: scenario.workspaceCanManage,
      rows,
    });
    if (route.request().method() !== "PUT") return json(route, workspacePolicy(workspaceRows));
    const body = route.request().postDataJSON() as { rows: PolicyRow[] };
    recorder.workspacePuts.push(body.rows);
    workspaceRows = body.rows;
    return json(route, workspacePolicy(workspaceRows));
  });

  return recorder;
}
