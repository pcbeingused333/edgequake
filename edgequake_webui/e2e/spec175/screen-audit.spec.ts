/**
 * SPEC-175 — screen audit. Captures every settings section the role may open, in light and dark
 * mode, plus a mobile pass. Screenshots go to specs/175-perfect-settings/e2e/screenshots/<ROUND>/
 * for visual review. Hermetic (mock-api project); assertions only confirm the screen rendered.
 */
import { expect, test } from "@playwright/test";
import { applyTheme, shot } from "./helpers/shots";
import { openSettings, settingsH1 as h1 } from "./helpers/open-settings";
import type { SettingsRole } from "./helpers/access-mock";

interface Screen {
  role: SettingsRole;
  path: string;
  slug: string;
  heading: string;
}

const SCREENS: Screen[] = [
  { role: "member", path: "/settings/you/appearance", slug: "you-appearance", heading: "Appearance" },
  { role: "member", path: "/settings/you/graph", slug: "you-graph", heading: "Graph display" },
  { role: "member", path: "/settings/you/browser-data", slug: "you-browser-data", heading: "Browser data" },
  { role: "member", path: "/settings/workspace/models", slug: "member-models", heading: "Models" },
  { role: "workspace_admin", path: "/settings/workspace/models", slug: "wsadmin-models", heading: "Models" },
  { role: "workspace_admin", path: "/settings/workspace/ingestion", slug: "wsadmin-ingestion", heading: "Ingestion" },
  { role: "workspace_admin", path: "/settings/workspace/access", slug: "wsadmin-access", heading: "Access" },
  { role: "super_admin", path: "/settings/organization/people", slug: "org-people", heading: "People" },
  { role: "platform_admin", path: "/settings/deployment/connections", slug: "deploy-connections", heading: "Connections" },
  { role: "platform_admin", path: "/settings/deployment/fleet", slug: "deploy-fleet", heading: "Fleet" },
  { role: "platform_admin", path: "/settings/deployment/observability", slug: "deploy-observability", heading: "Observability" },
];

test.describe("SPEC-175 screen audit @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(180_000));

  for (const screen of SCREENS) {
    test(`${screen.slug} light and dark @spec175`, async ({ page }) => {
      await openSettings(page, screen.role, screen.path);
      await expect(h1(page)).toHaveText(screen.heading);
      await page.waitForLoadState("networkidle").catch(() => undefined);
      await shot(page, screen.slug, { theme: "light", fullPage: true });
      await applyTheme(page, "dark");
      await shot(page, screen.slug, { theme: "dark", fullPage: true });
    });
  }

  test("mobile pass: models and people @spec175", async ({ page }) => {
    await openSettings(page, "workspace_admin", "/settings/workspace/models", 390);
    await shot(page, "mobile-wsadmin-models", { theme: "light", width: 390, fullPage: true });
    await openSettings(page, "super_admin", "/settings/organization/people", 390);
    await shot(page, "mobile-org-people", { theme: "light", width: 390, fullPage: true });
  });
});
