/**
 * Add-provider shapes cover the edgequake-llm catalog, including when the
 * shapes endpoint is down and the client fallback is used.
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";

test.describe("connection provider shapes @spec177", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the shape list includes Ollama, OpenRouter, and Mistral", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/connections");
    await page.route("**/api/v1/connections/shapes**", (route) => route.abort());
    await page.reload();
    await expect(h1(page)).toHaveText("Connections");
    const shape = page.getByTestId("connection-shape");
    await expect(shape.locator("option", { hasText: "Ollama" })).toHaveCount(1);
    await expect(shape.locator("option", { hasText: "OpenRouter" })).toHaveCount(1);
    await expect(shape.locator("option", { hasText: "Mistral" })).toHaveCount(1);
    await shape.selectOption("mistral");
    await expect(page.getByLabel("Base URL")).toHaveValue("https://api.mistral.ai/v1");
  });
});
