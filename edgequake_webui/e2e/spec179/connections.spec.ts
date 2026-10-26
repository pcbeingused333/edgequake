/**
 * SPEC-179 — Deployment › Connections: shape select, delete with impact. Hermetic.
 */
import { expect, test } from "@playwright/test";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";
import { installProviderFixture, type FixtureOptions } from "./helpers/fixtures";
import { applyTheme, setViewport, shot } from "./helpers/shots";

async function open(page: import("@playwright/test").Page, options: FixtureOptions = {}) {
  await openSettings(page, "platform_admin", "/settings/deployment/connections");
  const recorder = await installProviderFixture(page, options);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(h1(page)).toHaveText("Connections", { timeout: 30_000 });
  return recorder;
}

test.describe("SPEC-179 connections @spec179", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the shape select is never blank and rows say where the key lives", async ({ page }) => {
    await open(page);
    await expect(page.getByTestId("connection-shape")).toHaveValue("openai");
    await expect(page.getByTestId("connection-stored-in").first()).toContainText("Stored in Infisical");
    await shot(page, "deployment-connections-rich", { theme: "light", width: 1440 });
    await applyTheme(page, "dark");
    await shot(page, "deployment-connections-rich", { theme: "dark", width: 1440 });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await shot(page, "deployment-connections-rich", { theme: "light", width: 390 });
  });

  test("deleting a connection in use says how many assignments break", async ({ page }) => {
    const recorder = await open(page, { connectionImpactMessage: "3 assignments still use acme." });
    await page.getByTestId("connection-delete-acme").click();
    await expect(page.getByTestId("connection-delete-message")).toContainText("stored key is removed");
    await page.getByTestId("connection-delete-confirm").click();
    await expect(page.getByTestId("connection-delete-message")).toContainText("3 assignments still use acme.");
    await expect(page.getByTestId("connection-delete-confirm")).toHaveText("Delete anyway");
    await shot(page, "connection-delete-in-use", { theme: "light", width: 1440 });
    await page.getByTestId("connection-delete-confirm").click();
    await expect(page.getByTestId("connection-delete-acme")).toHaveCount(0);
    expect(recorder.connectionDeletes).toEqual([
      { id: "c-acme", confirm: false },
      { id: "c-acme", confirm: true },
    ]);
  });

  test("deleting an unused connection needs one confirmation and no window.confirm", async ({ page }) => {
    let nativeDialogs = 0;
    page.on("dialog", (dialog) => {
      nativeDialogs += 1;
      void dialog.dismiss();
    });
    const recorder = await open(page);
    await page.getByTestId("connection-delete-stale").click();
    await page.getByTestId("connection-delete-confirm").click();
    await expect(page.getByTestId("connection-delete-stale")).toHaveCount(0);
    expect(recorder.connectionDeletes).toEqual([{ id: "c-stale", confirm: false }]);
    expect(nativeDialogs).toBe(0);
  });

  test("cancel leaves the connection alone", async ({ page }) => {
    const recorder = await open(page);
    await page.getByTestId("connection-delete-stale").click();
    await page.getByTestId("connection-delete-dialog").getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByTestId("connection-delete-stale")).toBeVisible();
    expect(recorder.connectionDeletes).toEqual([]);
  });
});
