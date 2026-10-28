/**
 * SPEC-169 — Members, roles and SSO (hermetic). Every test asserts behaviour AND writes a
 * screenshot to specs/169-perfect-user-management/e2e/screenshots/<SPEC169_ROUND>/.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareMembersPage, type MembersMockOptions } from "./helpers/members-mock";
import { applyTheme, setViewport, shot } from "./helpers/shots";

const open = async (page: Page, opts: MembersMockOptions = {}, path = "/members") => {
  const state = await prepareMembersPage(page, opts);
  await setViewport(page, 1440);
  await page.goto(path, { waitUntil: "domcontentloaded" });
  return state;
};
const row = (page: Page, name: string) =>
  page.locator("[data-testid^=member-row-]").filter({ has: page.getByText(name, { exact: true }) });
const ready = async (page: Page) => {
  await expect(page.getByTestId("members-table")).toBeVisible({ timeout: 30_000 });
  await page.waitForTimeout(250);
};

test.describe("SPEC-169 members @spec169", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("directory overview — light, dark, mobile", async ({ page }) => {
    await open(page);
    await ready(page);
    await expect(page.getByTestId("members-summary")).toContainText("9 people in Acme Research");
    await expect(page.getByTestId("members-pending-count")).toHaveText("2 pending");
    await expect(page.getByTestId("members-seats")).toContainText("9 of 25 seats");
    await expect(row(page, "Ada Lovelace")).toContainText("You");
    await expect(page.getByTestId("signin-sso-keycloak").first()).toBeVisible();
    await shot(page, "members-overview", { theme: "light", width: 1440 });
    await applyTheme(page, "dark");
    await page.waitForTimeout(300);
    await shot(page, "members-overview", { theme: "dark", width: 1440 });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await page.waitForTimeout(300);
    await shot(page, "members-overview", { theme: "light", width: 390 });
  });

  test("filter, search, sort", async ({ page }) => {
    await open(page);
    await ready(page);
    await page.getByTestId("role-filter-readonly").click();
    await expect(page.locator("[data-testid^=member-row-][data-kind=member]")).toHaveCount(2);
    await shot(page, "members-filter-viewer");
    await page.getByTestId("role-filter-all").click();
    await page.getByTestId("members-search").fill("keycloak-none");
    await expect(page.getByTestId("members-no-match")).toBeVisible();
    await shot(page, "members-no-match");
    await page.getByRole("button", { name: /clear filters/i }).click();
    await page.getByTestId("members-search").fill("carol");
    await expect(page.locator("[data-testid^=member-row-]")).toHaveCount(1);
    await page.getByTestId("members-search").fill("");
    await page.getByTestId("sort-role").click();
    await expect(page.getByTestId("sort-role")).toBeVisible();
    await expect(page.locator("th[aria-sort=ascending]")).toBeVisible();
    await page.getByTestId("members-status-filter").click();
    await page.getByRole("option", { name: "Suspended" }).click();
    await expect(page.locator("[data-testid^=member-row-]")).toHaveCount(1);
    await expect(row(page, "Hiro Tanaka")).toBeVisible();
  });

  test("change a role (SSO pin note, last-owner lock)", async ({ page }) => {
    const mock = await open(page);
    await ready(page);
    await row(page, "Ada Lovelace").locator("[data-locked=true]").hover();
    await expect(page.getByRole("tooltip").first()).toContainText(/promote another owner/i);
    await shot(page, "members-last-owner-locked");
    await page.mouse.move(0, 0);

    const bob = row(page, "Bob Martin");
    await bob.locator("[data-testid^=role-select-]").click();
    await expect(page.getByText(/follows keycloak/i)).toBeVisible();
    await shot(page, "members-role-menu-sso");
    await page.keyboard.press("Escape");
    await expect(page.getByText(/follows keycloak/i)).toBeHidden();

    const diego = row(page, "Diego Alvarez");
    await diego.locator("[data-testid^=role-select-]").click();
    await page.getByTestId("role-option-admin").click();
    await expect(diego.getByTestId("role-badge-admin")).toBeVisible();
    expect(mock.calls.at(-1)).toMatchObject({ method: "PATCH", body: { role: "admin" } });
  });

  test("invite flow — validation, link shown once", async ({ page }) => {
    const mock = await open(page);
    await ready(page);
    await page.getByTestId("members-invite").click();
    await expect(page.getByTestId("invite-dialog")).toBeVisible();
    await page.getByTestId("invite-email").fill("not-an-email");
    await page.getByTestId("invite-submit").click();
    await expect(page.getByRole("alert")).toContainText(/valid email/i);
    await shot(page, "invite-dialog-invalid");
    await page.getByTestId("invite-email").fill("lena.fischer@acme.io");
    await page.getByTestId("invite-role-readonly").click();
    await shot(page, "invite-dialog-filled");
    await page.getByTestId("invite-submit").click();
    await expect(page.getByTestId("invite-link-panel")).toBeVisible();
    await expect(page.getByTestId("invite-link-input")).toHaveValue(/\/invite\/tok_/);
    expect(mock.calls.at(-1)).toMatchObject({ path: "invitations", body: { email: "lena.fischer@acme.io", role: "readonly" } });
    await shot(page, "invite-dialog-link");
    await page.getByTestId("invite-done").click();
    await expect(page.getByTestId("members-pending-count")).toHaveText("3 pending");
    await expect(row(page, "lena.fischer@acme.io")).toContainText("Invited by Ada Lovelace");
    await shot(page, "members-after-invite");
  });

  test("duplicate invite is explained", async ({ page }) => {
    await open(page);
    await ready(page);
    await page.getByTestId("members-invite").click();
    await page.getByTestId("invite-email").fill("carol.nguyen@acme.io");
    await page.getByTestId("invite-submit").click();
    await expect(page.getByText(/already a member/i).first()).toBeVisible();
  });

  test("remove an SSO member warns; revoke + resend invitations", async ({ page }) => {
    const mock = await open(page);
    await ready(page);
    const carol = row(page, "Carol Nguyen");
    await carol.locator("[data-testid^=member-actions-]").click();
    await page.getByTestId("action-remove").click();
    await expect(page.getByTestId("confirm-sso-warning")).toBeVisible();
    await shot(page, "confirm-remove-sso");
    await page.getByTestId("confirm-member-submit").click();
    await expect(carol).toHaveCount(0);
    expect(mock.calls.at(-1)).toMatchObject({ method: "DELETE" });

    const expired = row(page, "kara.thrace@acme.io");
    await expect(expired.getByTestId("status-expired")).toBeVisible();
    await expired.locator("[data-testid^=member-actions-]").click();
    await page.getByTestId("action-resend").click();
    await expect(page.getByTestId("invite-link-panel")).toBeVisible();
    await shot(page, "invite-resent-link");
    await page.getByTestId("invite-done").click();
    await expect(row(page, "kara.thrace@acme.io").getByTestId("status-invited")).toBeVisible();

    await row(page, "jules.verne@acme.io").locator("[data-testid^=member-actions-]").click();
    await page.getByTestId("action-revoke").click();
    await page.getByTestId("confirm-member-submit").click();
    await expect(row(page, "jules.verne@acme.io")).toHaveCount(0);
  });

  test("suspend and restore (instance admin)", async ({ page }) => {
    await open(page);
    await ready(page);
    await row(page, "Diego Alvarez").locator("[data-testid^=member-actions-]").click();
    await page.getByTestId("action-suspend").click();
    await shot(page, "confirm-suspend");
    await page.getByTestId("confirm-member-submit").click();
    await expect(row(page, "Diego Alvarez").getByTestId("status-deactivated")).toBeVisible();
    await row(page, "Diego Alvarez").locator("[data-testid^=member-actions-]").click();
    await page.getByTestId("action-suspend").click();
    await page.getByTestId("confirm-member-submit").click();
    await expect(row(page, "Diego Alvarez").getByTestId("status-active")).toBeVisible();
  });

  test("detail sheet", async ({ page }) => {
    await open(page);
    await ready(page);
    await row(page, "Bob Martin").getByRole("button", { name: "Bob Martin", exact: true }).click();
    await expect(page.getByTestId("member-detail-sheet")).toBeVisible();
    await expect(page.getByTestId("detail-sso-note")).toBeVisible();
    await page.waitForTimeout(400);
    await shot(page, "member-detail-sheet");
    await applyTheme(page, "dark");
    await page.waitForTimeout(300);
    await shot(page, "member-detail-sheet", { theme: "dark" });
  });

  test("roles & permissions tab", async ({ page }) => {
    await open(page);
    await page.getByTestId("tab-roles").click();
    await expect(page).toHaveURL(/tab=roles/);
    await expect(page.getByTestId("roles-matrix")).toBeVisible();
    await expect(page.getByTestId("cap-members_manage".replace("_", "."))).toBeVisible();
    await expect(page.getByTestId("cap-tenant.delete").locator("[data-granted=true]")).toHaveCount(1);
    await expect(page.getByTestId("instance-role-admin")).toBeVisible();
    await page.waitForTimeout(300);
    await shot(page, "roles-tab");
    await applyTheme(page, "dark");
    await page.waitForTimeout(300);
    await shot(page, "roles-tab", { theme: "dark" });
  });

  test("single sign-on tab", async ({ page }) => {
    await open(page, {}, "/members?tab=sso");
    await expect(page.getByTestId("idp-row-keycloak")).toBeVisible({ timeout: 30_000 });
    await page.waitForTimeout(300);
    await shot(page, "sso-tab");
  });

  test("states: restricted, error, alone", async ({ page }) => {
    await open(page, { viewerRole: null });
    await expect(page.getByTestId("members-restricted")).toBeVisible({ timeout: 30_000 });
    await shot(page, "state-restricted");
    await open(page, { fail: true });
    await expect(page.getByTestId("members-error")).toBeVisible({ timeout: 30_000 });
    await shot(page, "state-error");
    await open(page, { alone: true });
    await expect(page.getByTestId("members-alone")).toBeVisible({ timeout: 30_000 });
    await shot(page, "state-alone");
  });

  test("settings links to members; sidebar has Members", async ({ page }) => {
    await open(page, {}, "/settings/organization/people");
    await expect(page.getByTestId("members-link-card")).toBeAttached({ timeout: 30_000 });
    await expect(page.getByText("User Management", { exact: true })).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Members" }).first()).toBeVisible();
  });
});
