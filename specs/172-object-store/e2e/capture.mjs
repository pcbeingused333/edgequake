import { chromium } from "playwright";
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const out = path.join(here, "screenshots");
await mkdir(out, { recursive: true });

const base = process.env.FRONTEND_URL || "http://127.0.0.1:3010";
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

async function shot(name) {
  const file = path.join(out, name);
  await page.screenshot({ path: file, fullPage: false });
  console.log("wrote", file);
}

await page.goto(base + "/", { waitUntil: "networkidle" });
await page.waitForTimeout(1500);
const chip = page.getByTestId("original-store-mode");
try {
  await chip.waitFor({ timeout: 15000 });
  await chip.screenshot({ path: path.join(out, "01-originals-chip.png") });
  console.log("chip text:", await chip.innerText());
} catch (err) {
  console.log("chip missing:", err.message);
}
await shot("02-dashboard.png");

await page.goto(base + "/documents", { waitUntil: "networkidle" });
await page.waitForTimeout(1500);
await shot("03-documents.png");

await browser.close();
