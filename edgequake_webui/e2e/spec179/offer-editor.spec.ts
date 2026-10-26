/**
 * SPEC-179 — Organization and Workspace provider editors: revision on save,
 * confirm on the first narrowing, "+N" model overflow. Hermetic.
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";
import { installProviderFixture } from "./helpers/fixtures";
import { shot } from "./helpers/shots";

test.describe("SPEC-179 offer editors @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("an organization save sends the revision it read (If-Match)", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/providers");
    const recorder = await installProviderFixture(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(h1(page)).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("provider-offer-count")).toHaveText("1 of 2 providers");
    // The first provider shows its first four models and a +N overflow badge once there are more.
    await page.getByTestId("provider-toggle-acme").click();
    await expect(page.getByTestId("provider-offer-count")).toHaveText("2 of 2 providers");
    await shot(page, "org-providers-edit", { theme: "light", width: 1440 });
    await page.getByTestId("provider-offer-card").getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => recorder.offerPuts.length).toBe(1);
    expect(recorder.offerPuts[0]).toEqual({
      parent: "tenant",
      ifMatch: "3",
      body: { offer_mode: "constrained", slugs: ["acme", "openai"] },
    });
  });

  test("a workspace admin saves against the workspace revision", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    const recorder = await installProviderFixture(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(h1(page)).toBeVisible({ timeout: 30_000 });
    await page.getByTestId("provider-toggle-acme").click();
    await page.getByTestId("provider-offer-card").getByRole("button", { name: "Save", exact: true }).click();
    await expect.poll(() => recorder.offerPuts.length).toBe(1);
    expect(recorder.offerPuts[0]?.parent).toBe("workspace");
    expect(recorder.offerPuts[0]?.ifMatch).toBe("3");
  });

  test("a workspace offers 'every provider the organization offers', never 'parent'", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await installProviderFixture(page);
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByRole("button", { name: "Use every provider the organization offers" })).toBeVisible();
    await expect(page.getByText(/parent/i)).toHaveCount(0);
  });
});
