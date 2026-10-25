/**
 * SPEC-175 — shared page setup for the settings shell gates. Every spec opens the shell the
 * same way: hermetic data, the role's access payload, the slug lookup, then the route.
 */
import { expect, type Page } from "@playwright/test";
import { prepareSpec168Page } from "../../spec168/helpers/rich-mock";
import { mockSettingsAccess, mockWorkspaceBySlug, workspaceManageFor, type SettingsRole } from "./access-mock";
import { mockClearance, type ClearanceScenario } from "./clearance-mock";
import { mockLangfuseStatus } from "./observability-mock";
import { mockModelPolicy, type ModelPolicyRecorder, type ModelPolicyScenario } from "./model-policy-mock";
import { setViewport } from "./shots";

export const settingsH1 = (page: Page) =>
  page.getByTestId("settings-content").getByRole("heading", { level: 1 });

export const settingsNav = (page: Page) => page.getByTestId("settings-nav");

export const visibleBadge = (page: Page) =>
  page.locator('[data-testid="settings-access-badge"]:visible');

export const visibleSearch = (page: Page) =>
  page.locator('[data-testid="settings-search"]:visible');

/**
 * Opens a settings path as `role`. The model-policy and document-access fixtures are installed for
 * every page, so the typed editors are hermetic; pass `policy` to start from a specific ceiling,
 * narrowing or roles, and `clearance` to change the document-access state (for example, a failure).
 */
export async function openSettings(
  page: Page,
  role: SettingsRole,
  path: string,
  width = 1440,
  policy: Partial<ModelPolicyScenario> = {},
  clearance: Partial<ClearanceScenario> = {},
): Promise<ModelPolicyRecorder> {
  await prepareSpec168Page(page);
  await mockSettingsAccess(page, role);
  await mockWorkspaceBySlug(page);
  const recorder = await mockModelPolicy(page, { workspaceCanManage: workspaceManageFor(role), ...policy });
  await mockClearance(page, { canManage: workspaceManageFor(role), ...clearance });
  await mockLangfuseStatus(page);
  await setViewport(page, width);
  await page.goto(path, { waitUntil: "domcontentloaded" });
  await expect(settingsH1(page)).toBeVisible({ timeout: 30_000 });
  return recorder;
}
