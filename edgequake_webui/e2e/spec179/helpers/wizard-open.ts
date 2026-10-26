/**
 * SPEC-179 — open the Create / Reconfigure Workspace wizards on the hermetic fixture.
 */
import { expect, type Locator, type Page } from "@playwright/test";
import { openCreateWorkspaceDialog } from "../../helpers/spec013-bootstrap";
import { openSettings } from "../../spec175/helpers/open-settings";
import { installWizardFixture, type WizardFixtureOptions, type WizardRecorder } from "./wizard-fixture";

export type WizardRole = "super_admin" | "workspace_admin";

/** Hermetic shell as `role`, the wizard fixture on top, then the page under test. */
async function openFixturePage(page: Page, options: WizardFixtureOptions, role: WizardRole, path: string) {
  await openSettings(page, role, "/settings/workspace/models");
  const recorder = await installWizardFixture(page, options);
  if (path === "/settings/workspace/models") await page.reload({ waitUntil: "domcontentloaded" });
  else await page.goto(path, { waitUntil: "domcontentloaded" });
  return recorder;
}

/** Create Workspace wizard, open on its first step (name). */
export async function openCreateWizard(
  page: Page,
  options: WizardFixtureOptions = {},
  role: WizardRole = "super_admin",
): Promise<WizardRecorder> {
  const recorder = await openFixturePage(page, options, role, "/settings/workspace/models");
  await expect(page.locator('[data-testid="workspace-selector"]:visible').first()).toBeVisible({ timeout: 30_000 });
  await openCreateWorkspaceDialog(page);
  return recorder;
}

/** Reconfigure (Edit configuration) wizard, open on its first step (models). */
export async function openReconfigureWizard(
  page: Page,
  options: WizardFixtureOptions = {},
  role: WizardRole = "super_admin",
): Promise<WizardRecorder> {
  const recorder = await openFixturePage(page, options, role, "/workspace");
  const edit = page.getByTestId("workspace-edit-config");
  await expect(edit).toBeVisible({ timeout: 30_000 });
  await edit.click();
  await expect(page.getByTestId("reconfigure-workspace-wizard")).toBeVisible({ timeout: 15_000 });
  return recorder;
}

/** Click Next until `stepTestId` shows. Fails loudly if Next is ever disabled on the way. */
export async function nextUntil(page: Page, stepTestId: string, onStep?: (step: Locator) => Promise<void>) {
  for (let i = 0; i < 12; i += 1) {
    const step = page.getByTestId(stepTestId);
    if (await step.isVisible().catch(() => false)) return;
    const next = page.getByTestId("wizard-next");
    await expect(next).toBeEnabled();
    await onStep?.(page.getByTestId("wizard-step-body"));
    await next.click();
  }
  await expect(page.getByTestId(stepTestId)).toBeVisible();
}
