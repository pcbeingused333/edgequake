/**
 * Organization refresh reloads the ceiling. Connections update uses PUT.
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";

const CONN = "11111111-1111-4111-8111-111111111111";

test.describe("provider add update refresh @spec177", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("organization refresh reloads the model ceiling", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    let catalogHits = 0;
    const refreshes: string[] = [];
    await page.route("**/api/v1/models/discover/refresh", (route) => {
      refreshes.push(route.request().method());
      return route.fulfill({ json: { status: "ok", message: "ok" } });
    });
    await page.route("**/api/v1/tenants/*/model-catalog**", (route) => {
      catalogHits += 1;
      return route.fulfill({ json: { policy_mode: "unconstrained", entries: [] } });
    });
    await page.route(/\/api\/v1\/tenants\/[^/]+\/provider-offer\/?$/, (route) =>
      route.fulfill({
        json: {
          tenant_id: "00000000-0000-0000-0000-000000000002",
          offer_mode: "unconstrained",
          parent_count: 1,
          enabled_count: 1,
          can_manage: true,
          providers: [
            {
              slug: "ollama",
              display_name: "ollama",
              offered_by_parent: true,
              enabled: true,
              models: [{ id: "gemma4:latest", usages: ["query"] }],
            },
          ],
          legacy: [],
        },
      }),
    );
    await page.reload();
    await expect(h1(page)).toHaveText("Providers");
    const before = catalogHits;
    await page.getByTestId("org-refresh-models").click();
    await expect.poll(() => refreshes).toContain("POST");
    await expect.poll(() => catalogHits).toBeGreaterThan(before);
  });

  test("updating a stored connection sends PUT", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/connections");
    const puts: { base_url?: string }[] = [];
    await page.route("**/api/v1/connections/shapes**", (route) =>
      route.fulfill({
        json: [
          {
            id: "ollama",
            label: "Ollama",
            default_base_url: "http://127.0.0.1:11434",
            hint: "Local",
          },
        ],
      }),
    );
    await page.route("**/api/v1/connections**", async (route) => {
      const url = route.request().url();
      const method = route.request().method();
      if (url.includes("/shapes")) {
        await route.fallback();
        return;
      }
      if (method === "PUT") {
        puts.push(route.request().postDataJSON());
        await route.fulfill({
          json: {
            id: CONN,
            slug: "local",
            display_name: "local",
            api_shape: "ollama",
            base_url: "http://127.0.0.1:11435",
            source: "db",
            key_configured: true,
          },
        });
        return;
      }
      if (method === "GET") {
        await route.fulfill({
          json: [
            {
              id: CONN,
              slug: "local",
              display_name: "local",
              api_shape: "ollama",
              base_url: "http://127.0.0.1:11434",
              source: "db",
              key_configured: true,
            },
          ],
        });
        return;
      }
      await route.fallback();
    });
    await page.reload();
    await expect(h1(page)).toHaveText("Connections");
    await page.getByTestId("connection-update-local").click();
    await expect(page.getByLabel("Name")).toHaveValue("local");
    await page.getByLabel("Base URL").fill("http://127.0.0.1:11435");
    await page.getByTestId("connection-save").click();
    await expect.poll(() => puts.length).toBe(1);
    expect(puts[0]?.base_url).toBe("http://127.0.0.1:11435");
  });
});
