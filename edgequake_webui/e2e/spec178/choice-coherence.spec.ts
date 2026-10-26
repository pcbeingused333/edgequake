/**
 * SPEC-178 — the query chip lists eligible models, not the deployment inventory.
 * Hermetic (mock-api). Screenshots land in specs/178-provider-and-model-choice/e2e/screenshots.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec168Page } from "../spec168/helpers/rich-mock";
import { mockSettingsAccess, mockWorkspaceBySlug } from "../spec175/helpers/access-mock";
import { SPEC155_WORKSPACE } from "../spec155/helpers/mock-api";
import { mockClearance } from "../spec175/helpers/clearance-mock";
import { mockModelPolicy } from "../spec175/helpers/model-policy-mock";
import { mockLangfuseStatus } from "../spec175/helpers/observability-mock";
import { openSettings, settingsH1 as h1 } from "../spec175/helpers/open-settings";
import { applyTheme, setViewport, shot } from "./helpers/shots";

async function chooseEligible(page: Page, name: RegExp | string) {
  await page.getByTestId("query-model-chip").click();
  await expect(page.getByTestId("query-model-menu")).toBeVisible();
  await page.getByTestId("query-model-option").filter({ hasText: name }).click();
}

async function openQuery(
  page: Page,
  scenario: Parameters<typeof mockModelPolicy>[1] = { workspaceCanManage: false },
) {
  await prepareSpec168Page(page);
  await mockSettingsAccess(page, "member");
  await mockWorkspaceBySlug(page);
  await mockModelPolicy(page, scenario);
  await mockClearance(page, { canManage: false });
  await mockLangfuseStatus(page);
  await page.route("**/api/v1/models/llm**", (route) =>
    route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        default_provider: "openai",
        default_model: "inventory-only",
        models: [
          {
            provider: "openai",
            provider_display_name: "OpenAI",
            name: "inventory-only",
            display_name: "Inventory Only",
            capabilities: { supports_streaming: true, supports_vision: false },
          },
          {
            provider: "withdrawn",
            provider_display_name: "Withdrawn Co",
            name: "secret",
            display_name: "Withdrawn Co",
            capabilities: { supports_streaming: true, supports_vision: false },
            available: true,
          },
        ],
      }),
    }),
  );
  await setViewport(page, 1440);
  await page.goto("/w/spec155-ws/query", { waitUntil: "domcontentloaded" });
}

test.describe("SPEC-178 choice coherence @spec178", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the query chip hides an inventory-only model (EC-178-01)", async ({ page }) => {
    await openQuery(page);
    await expect(page.getByTestId("query-model-chip")).toBeVisible({ timeout: 30_000 });
    await page.getByTestId("query-model-chip").click();
    await expect(page.getByTestId("query-model-menu")).toBeVisible();
    await expect(page.getByTestId("query-model-menu").getByText("GPT-5 nano").first()).toBeVisible();
    await expect(page.getByText("Inventory Only")).toHaveCount(0);
    await expect(page.getByText("Withdrawn Co")).toHaveCount(0);
    await expect(page.getByTestId("query-model-option-default")).toContainText("Inherit");
    await expect(page.getByTestId("query-model-option-default")).toContainText("GPT-5 nano");
    await expect(page.getByTestId("query-model-chip")).toContainText("GPT-5 nano");
    await shot(page, "query-chip-open");
  });

  test("workspace models stay on the eligible editor (EC-178-23)", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models");
    await expect(h1(page)).toHaveText("Models");
    await expect(page.getByLabel("Model for Query")).toBeVisible();
    await shot(page, "wsadmin-models");
    await applyTheme(page, "dark");
    await shot(page, "wsadmin-models", { theme: "dark" });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await shot(page, "wsadmin-models", { width: 390 });
  });

  test("a locked query row does not open the menu (EC-178-10)", async ({ page }) => {
    await openQuery(page, { workspaceCanManage: false, lockQuery: true });
    const chip = page.getByTestId("query-model-chip");
    await expect(chip).toBeVisible({ timeout: 30_000 });
    await expect(chip).toHaveAttribute("data-locked", "true");
    await expect(chip).toContainText("Locked");
    await expect(chip).toContainText("gpt-5-nano");
    await chip.click({ force: true });
    await expect(page.getByTestId("query-model-menu")).toHaveCount(0);
    await shot(page, "query-chip-locked");
  });

  test("an empty eligible set disables Send (EC-178-18)", async ({ page }) => {
    await openQuery(page, {
      workspaceCanManage: false,
      workspaceRows: [
        { provider: "openai", model: "gpt-5-nano", usage: "query", enabled: false },
        { provider: "ollama", model: "gemma3", usage: "query", enabled: false },
      ],
    });
    await expect(page.getByTestId("query-choice-empty")).toBeVisible({ timeout: 30_000 });
    const input = page.getByRole("textbox").first();
    await input.fill("What is in the corpus?");
    await expect(page.getByTestId("query-send")).toBeDisabled();
    await page.getByTestId("query-model-chip").click();
    await expect(page.getByTestId("query-model-option")).toHaveCount(0);
    await shot(page, "query-choice-empty");
  });

  test("the stream switch follows the eligible bit (EC-178-33)", async ({ page }) => {
    await openQuery(page, { workspaceCanManage: false, supportsStreaming: false });
    await page.getByTestId("query-model-chip").click();
    await expect(page.getByTestId("query-model-stream-toggle")).toBeDisabled();
    await expect(page.getByText("no stream").first()).toBeVisible();
    await shot(page, "query-chip-no-stream");
  });

  test("two workspace routes keep two slots (EC-178-05)", async ({ page }) => {
    const other = {
      ...SPEC155_WORKSPACE,
      id: "spec178-ws-002",
      slug: "spec178-other",
      name: "Other workspace",
    };
    await openQuery(page);
    await page.route("**/api/v1/tenants/*/workspaces**", (route) => {
      const url = route.request().url();
      if (url.includes("/by-slug/spec178-other")) {
        return route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(other) });
      }
      if (url.includes("/by-slug/")) {
        return route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(SPEC155_WORKSPACE),
        });
      }
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({ items: [SPEC155_WORKSPACE, other], total: 2, offset: 0, limit: 20 }),
      });
    });
    await page.route(new RegExp(`/api/v1/workspaces/${other.id}(\\?.*)?$`), (route) =>
      route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify(other) }),
    );
    await chooseEligible(page, /Gemma 3/);
    await expect(page.getByTestId("query-model-chip")).toContainText("Gemma 3");
    const stored = await page.evaluate(() => {
      const raw = localStorage.getItem("edgequake-settings");
      return raw ? JSON.parse(raw) : null;
    });
    expect(stored?.state?.queryModelsByWorkspace?.[SPEC155_WORKSPACE.id]).toEqual({
      provider: "ollama",
      model: "gemma3",
    });
    await page.goto("/w/spec178-other/query", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("query-model-chip")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("query-model-chip")).toContainText("GPT-5 nano");
    await expect(page.getByTestId("query-model-chip")).not.toContainText("Gemma 3");
    await page.goto("/w/spec155-ws/query", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("query-model-chip")).toContainText("Gemma 3", { timeout: 30_000 });
  });

  test("a ceiling refetch clears a pair the list dropped (EC-178-28)", async ({ page }) => {
    await openQuery(page);
    await chooseEligible(page, /Gemma 3/);
    await expect(page.getByTestId("query-model-chip")).toContainText("Gemma 3");
    await page.route("**/api/v1/models/eligible**", (route) => {
      const usage = new URL(route.request().url()).searchParams.get("usage");
      return route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          usage,
          policy_mode: "constrained",
          models: usage === "query"
            ? [{ provider: "openai", provider_display_name: "OpenAI", model: "gpt-5-nano", display_name: "GPT-5 nano", supports_vision: false, supports_streaming: true }]
            : [],
        }),
      });
    });
    await page.reload({ waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("query-model-chip")).toContainText("GPT-5 nano", { timeout: 30_000 });
    await expect(page.getByTestId("query-model-chip")).not.toContainText("Gemma 3");
  });
});
