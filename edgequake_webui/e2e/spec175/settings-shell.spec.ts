/**
 * SPEC-175 — settings shell: role-gated navigation, deeplinks, redirects, search,
 * keyboard, and accessibility. Hermetic (mock-api project). Every test writes screenshots
 * to specs/175-perfect-settings/e2e/screenshots/<SPEC175_ROUND>/ for the visual loop.
 */
import { expect, test, type Page } from "@playwright/test";
import { SPEC155_WORKSPACE } from "../spec155/helpers/mock-api";
import { type SettingsRole } from "./helpers/access-mock";
import { applyTheme, setViewport, shot } from "./helpers/shots";
import { openSettings, settingsH1 as h1, settingsNav as nav } from "./helpers/open-settings";

test.describe("SPEC-175 settings shell @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("member sees You and Workspace only, with a Member badge", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/appearance");
    await expect(h1(page)).toHaveText("Appearance");
    await expect(nav(page).locator('[data-scope="you"]')).toBeVisible();
    await expect(nav(page).locator('[data-scope="workspace"]')).toBeVisible();
    await expect(nav(page).locator('[data-scope="organization"]')).toHaveCount(0);
    await expect(nav(page).locator('[data-scope="deployment"]')).toHaveCount(0);
    await expect(page.locator('[data-testid="settings-access-badge"]:visible')).toHaveAttribute("data-chip", "member");
    await shot(page, "member-appearance");
  });

  test("workspace admin does not see Organization (EC-175-34)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(nav(page).locator('[data-scope="organization"]')).toHaveCount(0);
    await expect(nav(page).locator('[data-testid="settings-nav-ingestion"]')).toBeVisible();
    await expect(page.locator('[data-testid="settings-access-badge"]:visible')).toHaveAttribute("data-chip", "workspace_admin");
    await shot(page, "wsadmin-models");
  });

  test("platform admin badge says Platform admin, never Super admin (LAW-175-17)", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment/fleet");
    await expect(page.locator('[data-testid="settings-access-badge"]:visible')).toHaveText("Platform admin");
    await expect(nav(page).locator('[data-scope="deployment"]')).toBeVisible();
    await expect(nav(page).locator('[data-scope="organization"]')).toBeVisible();
    await shot(page, "platform-fleet");
  });

  test("a member opening an editor section is redirected to Models (EC-175-34)", async ({ page }) => {
    await openSettings(page, "member", "/settings/workspace/ingestion");
    await expect(page).toHaveURL(/\/settings\/workspace\/models$/);
    await expect(h1(page)).toHaveText("Models");
  });

  test("a bare scope path canonicalizes to its first section", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/deployment");
    await expect(page).toHaveURL(/\/settings\/deployment\/connections$/);
  });

  test("the workspace deeplink shows Models, not /workspace (W-31)", async ({ page }) => {
    await openSettings(page, "workspace_admin", `/w/${SPEC155_WORKSPACE.slug}/settings`);
    await expect(page).toHaveURL(new RegExp(`/w/${SPEC155_WORKSPACE.slug}/settings$`));
    await expect(h1(page)).toHaveText("Models");
    await expect(nav(page).getByTestId("settings-nav-models")).toHaveAttribute("aria-current", "page");
    await shot(page, "deeplink-models");
  });

  test("an unknown section shows its own empty state, not a crash", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/nope");
    await expect(page.getByTestId("settings-unknown")).toBeVisible();
  });

  test("the sections are reachable from the nav and the h1 moves focus", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/you/appearance");
    await nav(page).getByTestId("settings-nav-browser-data").click();
    await expect(page).toHaveURL(/\/settings\/you\/browser-data$/);
    await expect(h1(page)).toHaveText("Browser data");
    await expect(h1(page)).toBeFocused();
  });

  test("slash focuses search; a query finds a section; Enter opens it", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/you/appearance");
    await page.locator("body").click({ position: { x: 5, y: 5 } });
    await page.keyboard.press("/");
    const search = page.locator('[data-testid="settings-search"]:visible');
    await expect(search).toBeFocused();
    await search.fill("pdf");
    await expect(page.getByTestId("settings-search-results")).toContainText("Ingestion");
    await shot(page, "search-pdf");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/settings\/workspace\/ingestion$/);
  });

  test("search never offers a section the caller cannot open (EC-175-43)", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/appearance");
    const search = page.locator('[data-testid="settings-search"]:visible');
    await search.fill("pdf");
    await expect(page.getByTestId("settings-search-empty")).toHaveText("No setting named that.");
    await expect(page.getByTestId("settings-search-results")).toHaveCount(0);
    await shot(page, "search-empty-member");
  });

  test("Escape clears search and the arrow keys move through results", async ({ page }) => {
    await openSettings(page, "platform_admin", "/settings/you/appearance");
    const search = page.locator('[data-testid="settings-search"]:visible');
    await search.fill("e");
    const options = page.getByRole("option");
    await expect(options.first()).toBeVisible();
    await search.press("ArrowDown");
    await expect(options.first()).toHaveAttribute("aria-selected", "true");
    await search.press("Escape");
    await expect(search).toHaveValue("");
  });

  test("nav uses roving arrow keys between links", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/appearance");
    await nav(page).getByTestId("settings-nav-appearance").focus();
    await page.keyboard.press("ArrowDown");
    await expect(nav(page).getByTestId("settings-nav-graph")).toBeFocused();
    await page.keyboard.press("End");
    await expect(nav(page).getByTestId("settings-nav-models")).toBeFocused();
    await page.keyboard.press("Home");
    await expect(nav(page).getByTestId("settings-nav-appearance")).toBeFocused();
  });

  test("theme cards are a radio group named by its label", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/appearance");
    const group = page.getByRole("radiogroup", { name: "Theme" });
    await expect(group).toBeVisible();
    await page.getByRole("radio", { name: "Dark" }).click();
    await expect(page.locator("html")).toHaveClass(/dark/);
    await shot(page, "appearance-dark-selected", { theme: "dark" });
  });

  test("clearing history asks first and Escape keeps the data", async ({ page }) => {
    await openSettings(page, "member", "/settings/you/browser-data");
    await page.getByTestId("clear-history-trigger").click();
    const dialog = page.getByRole("alertdialog");
    await expect(dialog).toBeVisible();
    await shot(page, "browser-data-clear-confirm");
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(page.getByTestId("clear-history-trigger")).toBeFocused();
  });

  test("mobile shows the selects instead of the nav at 390px", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/you/appearance", 390);
    await expect(page.getByTestId("settings-mobile-nav")).toBeVisible();
    await expect(page.getByTestId("settings-mobile-scope")).toBeVisible();
    await shot(page, "mobile-appearance", { width: 390 });
  });

  test("dark theme renders the shell", async ({ page }) => {
    await openSettings(page, "super_admin", "/settings/organization/people");
    await applyTheme(page, "dark");
    await shot(page, "people-dark", { theme: "dark" });
  });
});
