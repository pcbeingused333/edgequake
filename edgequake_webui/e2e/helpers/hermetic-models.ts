/**
 * Hermetic answers for the model offer the wizard and the query page judge.
 *
 * A generic `GET /api/v1/models` mock also matches `/models/eligible`. The
 * wizard then sees an empty list and keeps Next disabled. Register this
 * AFTER that generic route so the specific path wins.
 */
import type { Page } from "@playwright/test";

export async function mockHermeticModelOffer(
  page: Page,
  models: { llm?: string; embedding?: string } = {},
): Promise<void> {
  const llm = models.llm ?? "gemma4:latest";
  const embedding = models.embedding ?? "embeddinggemma:latest";
  await page.route("**/api/v1/models/eligible**", (route) => {
    const usage = new URL(route.request().url()).searchParams.get("usage") ?? "query";
    const offered =
      usage === "embedding"
        ? [
            {
              provider: "ollama",
              provider_display_name: "Ollama",
              model: embedding,
              display_name: embedding,
              supports_vision: false,
              supports_streaming: false,
              embedding_dimension: 768,
            },
          ]
        : [
            {
              provider: "ollama",
              provider_display_name: "Ollama",
              model: llm,
              display_name: llm,
              supports_vision: true,
              supports_streaming: true,
            },
          ];
    return route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        usage,
        policy_mode: "unconstrained",
        models: offered,
      }),
    });
  });
  await page.route("**/api/v1/decision/**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({ status: "disabled", available: false, models: [] }),
    }),
  );
}
