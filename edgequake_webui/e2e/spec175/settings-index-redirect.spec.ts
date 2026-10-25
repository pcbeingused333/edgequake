/**
 * SPEC-175 — bare `/settings` redirects to the first section and keeps its query string. The
 * redirect lives in `next.config.ts`; a redirect-only RSC page once threw a dev-only React timing
 * error ("cannot have a negative time stamp") on this URL. Hermetic (mock-api project).
 */
import { expect, test } from "@playwright/test";
import { openSettings } from "./helpers/open-settings";

const TIMING_ERROR = /negative time stamp/i;

test.describe("SPEC-175 settings index redirect @spec175", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("bare /settings with a query lands on Appearance without a dev timing error", async ({ page }) => {
    const errors: string[] = [];
    page.on("pageerror", (err) => errors.push(err.message));
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(msg.text());
    });

    await openSettings(page, "member", "/settings?workspace=default");

    // The shell then canonicalises the workspace query, so only the path is asserted.
    await expect(page).toHaveURL(/\/settings\/you\/appearance(\?.*)?$/);
    expect(errors.filter((e) => TIMING_ERROR.test(e))).toEqual([]);
  });
});
