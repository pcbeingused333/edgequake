/**
 * Fleet refresh: every refreshable offered slug can pull a live list and
 * confirm it into connection_models. Hermetic.
 */
import { expect, test, type Page } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";

function deployment() {
  return {
    providers: [
      {
        slug: "ollama",
        display_name: "Ollama",
        origin: "fleet",
        api_shape: "ollama",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        models: [],
      },
      {
        slug: "openai",
        display_name: "OpenAI",
        origin: "fleet",
        api_shape: "openai_chat",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        models: [],
      },
      {
        slug: "acme",
        display_name: "Acme",
        origin: "fleet",
        api_shape: "openai_chat",
        offered: true,
        withdrawn: false,
        probe: "ok",
        key_configured: true,
        models: [{ id: "acme-chat", usages: ["query"] }],
      },
    ],
  };
}

async function install(page: Page) {
  const refreshes: string[] = [];
  const modelPuts: unknown[] = [];
  await page.route("**/api/v1/settings/providers**", (route) =>
    route.fulfill({
      json: {
        llm_providers: [
          { id: "ollama", name: "Ollama", refreshable: true, available: true, description: "", config_requirements: [], default_models: { chat_model: "gemma4:latest", embedding_model: "", embedding_dimension: 0 } },
          { id: "openai", name: "OpenAI", refreshable: true, available: true, description: "", config_requirements: [], default_models: { chat_model: "gpt-5-nano", embedding_model: "", embedding_dimension: 0 } },
          { id: "acme", name: "Acme", refreshable: false, available: true, description: "", config_requirements: [], default_models: { chat_model: "", embedding_model: "", embedding_dimension: 0 } },
        ],
        embedding_providers: [],
        active_llm_provider: "ollama",
        active_embedding_provider: "ollama",
      },
    }),
  );
  await page.route(/\/api\/v1\/deployment\/providers\/?$/, (route) =>
    route.fulfill({ json: deployment() }),
  );
  await page.route(/\/api\/v1\/deployment\/providers\/[^/]+\/refresh$/, (route) =>
    route.fulfill({
      json: { status: "ok", error: null, added: [], removed: [], refreshed_at: "2026-10-26T00:00:00Z" },
    }),
  );
  await page.route("**/api/v1/models/discover/refresh", (route) => {
    refreshes.push(route.request().method());
    return route.fulfill({ json: { status: "ok", message: "invalidated" } });
  });
  await page.route(/\/api\/v1\/models\/llm/, (route) => {
    const provider = new URL(route.request().url()).searchParams.get("provider");
    const models =
      provider === "ollama"
        ? [{ provider: "ollama", provider_display_name: "Ollama", name: "gemma4:latest", display_name: "Gemma 4", model_type: "llm", description: "", deprecated: false, capabilities: { supports_vision: false, supports_streaming: true, context_length: 8192, max_output_tokens: 2048, supports_function_calling: false, supports_json_mode: false, supports_system_message: true, embedding_dimension: 0 }, cost: { input_per_1k: 0, output_per_1k: 0 }, tags: [], discovery_source: "dynamic_api" }]
        : provider === "openai"
          ? [{ provider: "openai", provider_display_name: "OpenAI", name: "gpt-5-nano", display_name: "GPT-5 nano", model_type: "llm", description: "", deprecated: false, capabilities: { supports_vision: false, supports_streaming: true, context_length: 128000, max_output_tokens: 4096, supports_function_calling: true, supports_json_mode: true, supports_system_message: true, embedding_dimension: 0 }, cost: { input_per_1k: 0, output_per_1k: 0 }, tags: [], discovery_source: "hybrid" }]
          : [];
    return route.fulfill({
      json: { models, default_provider: "ollama", default_model: "gemma4:latest" },
    });
  });
  await page.route(/\/api\/v1\/deployment\/providers\/[^/]+\/models$/, async (route) => {
    modelPuts.push(route.request().postDataJSON());
    await route.fulfill({ status: 204, body: "" });
  });
  return { refreshes, modelPuts };
}

test.describe("fleet model refresh @spec177", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("refreshable fleet providers list live models and save the selection", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/providers");
    const { refreshes, modelPuts } = await install(page);
    await page.reload();
    await expect(h1(page)).toHaveText("Providers");
    await expect(page.getByTestId("fleet-refresh-ollama")).toBeVisible();
    await expect(page.getByTestId("fleet-refresh-openai")).toBeVisible();
    await expect(page.getByTestId("fleet-refresh-acme")).toHaveCount(0);

    await page.getByTestId("fleet-refresh-ollama").click();
    await expect(page.getByTestId("fleet-discovered-ollama")).toContainText("Gemma 4");
    expect(refreshes).toContain("POST");
    await page.getByRole("checkbox", { name: "Gemma 4" }).click();
    // Model edits are a draft until Save is pressed (no autosave).
    await page.getByTestId("fleet-models-ollama").getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => modelPuts.length).toBeGreaterThan(0);
    expect(modelPuts.at(-1)).toEqual({
      models: [
        {
          id: "gemma4:latest",
          supports_chat: true,
          supports_vision: false,
          supports_embedding: false,
        },
      ],
    });

    await page.getByTestId("fleet-refresh-openai").click();
    await expect(page.getByTestId("fleet-discovered-openai")).toContainText("GPT-5 nano");
  });
});
