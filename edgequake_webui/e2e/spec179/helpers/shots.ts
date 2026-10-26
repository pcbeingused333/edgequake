/**
 * SPEC-179 screenshot helper. Writes under
 * specs/179-provider-and-model-choice-perfect/e2e/screenshots/{SPEC179_ROUND}/.
 */
import fs from "node:fs";
import path from "node:path";
import type { Locator, Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC179_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/179-provider-and-model-choice-perfect/e2e/screenshots",
);
export const ROUND = process.env.SPEC179_ROUND ?? "round-01";

/** The page scrolls inside a container, so fullPage alone clips. Grow the viewport to fit it. */
async function contentHeight(page: Page): Promise<number> {
  return page.evaluate(() => {
    let max = document.documentElement.scrollHeight;
    for (const el of Array.from(document.querySelectorAll<HTMLElement>("main, [data-testid='settings-content'], [data-radix-scroll-area-viewport]"))) {
      max = Math.max(max, el.scrollHeight + el.getBoundingClientRect().top);
    }
    return Math.ceil(max);
  });
}

export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number; fullPage?: boolean } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const dir = path.join(SPEC179_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `${name}-${theme}-${width}.png`);
  const original = page.viewportSize();
  if (original && (opts.fullPage ?? true)) {
    const height = Math.min(Math.max(original.height, await contentHeight(page)), 2600);
    await page.setViewportSize({ width: original.width, height });
    await page.waitForTimeout(150);
  }
  await page.screenshot({
    path: file,
    fullPage: opts.fullPage ?? true,
    animations: "disabled",
    caret: "hide",
    timeout: 15_000,
  });
  if (original) await page.setViewportSize(original);
  return file;
}

/** Screenshot one element (a dialog) under the round folder, same naming as `shot`. */
export async function shotLocator(
  page: Page,
  target: Locator,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const dir = path.join(SPEC179_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `${name}-${theme}-${width}.png`);
  await target.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 15_000 });
  return file;
}
