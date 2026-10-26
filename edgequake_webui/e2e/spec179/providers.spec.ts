/**
 * SPEC-179 — the offer editor tells the truth when the load fails, and choice
 * screens do not search the raw inventory.
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";
import { installProviderFixture } from "./helpers/fixtures";
import { applyTheme, setViewport, shot } from "./helpers/shots";

test.describe("SPEC-179 providers @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("offer-error-state", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    await page.route("**/api/v1/tenants/**/provider-offer**", (route) =>
      route.fulfill({ status: 500, contentType: "application/json", body: "{}" }),
    );
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(h1(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("provider-offer-lead")).toContainText("Could not load providers");
    await expect(page.getByTestId("provider-offer-retry")).toBeVisible();
    await expect(page.getByText("no providers yet")).toHaveCount(0);
    await shot(page, "org-offer-error");
  });

  test("choice-does-not-search-inventory", async ({ page }) => {
    const searches: string[] = [];
    await page.route("**/api/v1/models/search**", (route) => {
      searches.push(route.request().url());
      return route.fulfill({ json: { models: [{ provider: "openai", id: "inventory-only", name: "Inventory Only" }] } });
    });
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(h1(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByText("Inventory Only")).toHaveCount(0);
    expect(searches).toEqual([]);
    await shot(page, "workspace-models");
  });

  test("viewer-readonly", async ({ page }) => {
    await openSettings(page, "member", "/settings/workspace/models");
    await installProviderFixture(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(h1(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("provider-toggle-openai")).toBeDisabled();
    await expect(page.getByRole("button", { name: "Save", exact: true })).toHaveCount(0);
    await shot(page, "workspace-models-readonly");
  });
});

test.describe("SPEC-179 screens @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(180_000));

  test("capture provider and model screens", async ({ page }) => {
    const frames: { name: string; path: string; role: "platform_admin" | "super_admin" | "workspace_admin"; theme: "light" | "dark"; width: number }[] = [
      { name: "deployment-providers", path: "/settings/deployment/providers", role: "platform_admin", theme: "light", width: 1440 },
      { name: "deployment-providers", path: "/settings/deployment/providers", role: "platform_admin", theme: "dark", width: 1440 },
      { name: "deployment-providers", path: "/settings/deployment/providers", role: "platform_admin", theme: "light", width: 390 },
      { name: "deployment-connections", path: "/settings/deployment/connections", role: "platform_admin", theme: "light", width: 1440 },
      { name: "org-providers", path: "/settings/organization/providers", role: "super_admin", theme: "light", width: 1440 },
      { name: "org-providers", path: "/settings/organization/providers", role: "super_admin", theme: "dark", width: 1440 },
      { name: "workspace-models", path: "/settings/workspace/models", role: "workspace_admin", theme: "light", width: 1440 },
      { name: "workspace-models", path: "/settings/workspace/models", role: "workspace_admin", theme: "dark", width: 1440 },
      { name: "workspace-models", path: "/settings/workspace/models", role: "workspace_admin", theme: "light", width: 390 },
    ];
    for (const frame of frames) {
      await openSettings(page, frame.role, frame.path, frame.width);
      await installProviderFixture(page);
      await page.reload({ waitUntil: "domcontentloaded" });
      await expect(h1(page)).toBeVisible({ timeout: 30_000 });
      await applyTheme(page, frame.theme);
      await setViewport(page, frame.width);
      await expect(h1(page)).toBeVisible({ timeout: 30_000 });
      await shot(page, frame.name, { theme: frame.theme, width: frame.width });
    }
  });
});
