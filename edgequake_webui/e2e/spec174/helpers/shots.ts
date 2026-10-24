/**
 * SPEC-174 screenshot helper. Writes under specs/174-document-security/e2e/screenshots/{round}/.
 */
import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC174_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/174-document-security/e2e/screenshots",
);

export const ROUND = process.env.SPEC174_ROUND ?? "round-03";

export function shotPath(name: string, theme: Theme, width: number): string {
  const dir = path.join(SPEC174_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number; fullPage?: boolean } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = shotPath(name, theme, width);
  await page.screenshot({
    path: file,
    fullPage: opts.fullPage ?? false,
    animations: "disabled",
    caret: "hide",
    timeout: 15_000,
  });
  return file;
}
