/**
 * SPEC-175 — gate rows from specs/175-perfect-settings/13-e2e-test-matrix.md that the shell
 * must hold on every change: one health region, scope hidden by URL and search, and the
 * models matrix read-only unless the caller manages the selected workspace.
 * Hermetic (mock-api project). Screenshots go to the round folder for visual review.
 */
import { expect, test } from "@playwright/test";
import { shot } from "./helpers/shots";
import {
  openSettings,
  settingsH1 as h1,
  settingsNav as nav,
  visibleSearch,
} from "./helpers/open-settings";

test.describe("SPEC-175 settings gates @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("one provider health region on the fleet page (EC-175-29)", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/fleet");
    await expect(h1(page)).toHaveText("Fleet");
    const region = page.getByTestId("settings-content").getByText("Provider Status", { exact: true });
    await expect(region).toHaveCount(1);
  });

  test("a workspace admin cannot reach Deployment by URL or search (EC-175-34)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/deployment/fleet");
    await expect(page).toHaveURL(/\/settings\/you\/appearance$/);
    await expect(nav(page).locator('[data-scope="deployment"]')).toHaveCount(0);

    await visibleSearch(page).fill("fleet");
    await expect(page.getByTestId("settings-search-empty")).toBeVisible();
    await expect(page.getByTestId("settings-search-results").getByText("Fleet", { exact: true })).toHaveCount(0);
    await shot(page, "wsadmin-deployment-redirect");
  });

  test("a member cannot reach Organization by URL or search (EC-175-43)", async ({ page }) => {
    await openSettings(page, "member", "/settings/organization/people");
    await expect(page).toHaveURL(/\/settings\/you\/appearance$/);
    await expect(nav(page).locator('[data-scope="organization"]')).toHaveCount(0);

    await visibleSearch(page).fill("people");
    await expect(page.getByTestId("settings-search-empty")).toBeVisible();
    await shot(page, "member-organization-redirect");
  });

  test("a member reads the model roles and cannot save them (EC-175-16, EC-175-31)", async ({ page }) => {
    await openSettings(page, "member", "/settings/workspace/models");
    await expect(h1(page)).toHaveText("Models");
    await expect(page.getByTestId("model-roles-readonly")).toBeVisible();
    const roles = page.getByTestId("model-roles-card");
    await expect(roles.getByRole("button", { name: "Save", exact: true })).toHaveCount(0);
    await expect(page.getByLabel("Model for Query")).toBeDisabled();
    await shot(page, "member-models-readonly");
  });

  test("a workspace admin edits the model roles", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(page.getByTestId("model-roles-readonly")).toHaveCount(0);
    await expect(page.getByLabel("Model for Query")).toBeEnabled();
    await shot(page, "wsadmin-models-editable");
  });

  test("a workspace admin of another workspace sees this one read-only (EC-175-39)", async ({ page }) => {
    await openSettings(page, "other_workspace_admin", "/settings/workspace/models");
    await expect(h1(page)).toHaveText("Models");
    await expect(page.getByTestId("model-roles-readonly")).toBeVisible();
    await expect(page.getByTestId("model-roles-card").getByRole("button", { name: "Save", exact: true })).toHaveCount(0);
    await shot(page, "other-workspace-models-readonly");
  });
});
