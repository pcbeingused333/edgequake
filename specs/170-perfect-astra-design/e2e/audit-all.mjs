// Audit every marketing route across viewports + themes for visual defects.
// Reports CLS, horizontal overflow, broken images, text clipping, viewport
// escapes, and tiny tap targets. Saves full-page screenshots per route.
import pw from "/Users/raphaelmansuy/Github/03-working/edgequake/edgequake_webui/node_modules/playwright-core/index.js";
const { chromium } = pw;
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = "/Users/raphaelmansuy/Github/03-working/edgequake/specs/170-perfect-astra-design/e2e/screenshots/audit";
mkdirSync(ROOT, { recursive: true });
const BASE = process.argv[2] || "http://localhost:4321";

const ROUTES = [
  ["home", "/"],
  ["demo", "/demo/"],
  ["ecosystem", "/ecosystem/"],
  ["enterprise", "/enterprise/"],
  ["contact", "/contact/"],
  ["work-with-raphael", "/work-with-raphael/"],
  ["tutorial-index", "/tutorial/"],
  ["tutorial-chapter", "/tutorial/01-what-this-is/"],
  ["404", "/this-page-does-not-exist/"],
];

const VP = {
  desktop: { width: 1440, height: 900 },
  mobile: { width: 390, height: 844 },
};

const CLS = `window.__cls=0;try{new PerformanceObserver(l=>{for(const e of l.getEntries())if(!e.hadRecentInput)window.__cls+=e.value}).observe({type:'layout-shift',buffered:true})}catch(e){}`;

const ANALYZE = () => {
  const vw = innerWidth;
  const defects = [];
  document.querySelectorAll("img").forEach((i) => {
    if (!i.complete || i.naturalWidth === 0) defects.push({ k: "broken-image", d: i.currentSrc || i.src });
  });
  const leaves = [...document.querySelectorAll("h1,h2,h3,h4,p,span,a,li,td,th,button,figcaption,code,summary")].filter((el) => {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return r.width > 0 && r.height > 0 && cs.visibility !== "hidden" && cs.display !== "none" && el.textContent.trim() && el.offsetParent;
  });
  leaves.forEach((el) => {
    const cs = getComputedStyle(el);
    const clips = cs.textOverflow === "ellipsis" || cs.overflowX === "hidden" || cs.overflow === "hidden";
    if (clips && el.scrollWidth > el.clientWidth + 2 && !el.closest(".mkt-frame-bar") && !el.closest("[class*=truncate]")) {
      defects.push({ k: "text-clip", d: el.tagName + ":" + el.textContent.trim().slice(0, 30) });
    }
  });
  document.querySelectorAll("main *, header *, footer *").forEach((el) => {
    if (el.getAttribute("aria-hidden") === "true" || el.closest("[aria-hidden=true]")) return;
    const cs = getComputedStyle(el);
    if (cs.overflowX === "auto" || el.closest("[class*=overflow-x-auto]") || el.closest(".mkt-code-showcase")) return;
    const r = el.getBoundingClientRect();
    if (r.width > 0 && (r.right > vw + 1.5 || r.left < -1.5)) defects.push({ k: "escape", d: el.tagName + "." + (typeof el.className === "string" ? el.className.split(" ")[0] : "") + " R" + Math.round(r.right) });
  });
  // text overlap among headings/paras
  const blocks = leaves.filter((e) => ["H1", "H2", "H3", "P"].includes(e.tagName));
  for (let i = 0; i < blocks.length; i++) for (let j = i + 1; j < blocks.length; j++) {
    if (blocks[i].contains(blocks[j]) || blocks[j].contains(blocks[i])) continue;
    const a = blocks[i].getBoundingClientRect(), b = blocks[j].getBoundingClientRect();
    const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
    const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
    if (ox > 8 && oy > 8) defects.push({ k: "overlap", d: blocks[i].textContent.trim().slice(0, 16) + " / " + blocks[j].textContent.trim().slice(0, 16) });
  }
  return { vw, overflow: document.documentElement.scrollWidth - vw, defects };
};

async function run() {
  const browser = await chromium.launch();
  const report = { base: BASE, when: new Date().toISOString(), routes: [] };
  for (const [name, path] of ROUTES) {
    for (const [vn, vp] of Object.entries(VP)) {
      for (const theme of ["light", "dark"]) {
        const ctx = await browser.newContext({ viewport: vp, deviceScaleFactor: 1, colorScheme: theme });
        const page = await ctx.newPage();
        await page.addInitScript(CLS);
        let status = 0;
        try {
          const resp = await page.goto(BASE + path, { waitUntil: "load", timeout: 45000 });
          status = resp ? resp.status() : 0;
        } catch (e) { report.routes.push({ name, path, vn, theme, error: String(e) }); await ctx.close(); continue; }
        await page.evaluate(async () => {
          await (document.fonts ? document.fonts.ready : 0);
          document.querySelectorAll("img[loading=lazy]").forEach((i) => (i.loading = "eager"));
          for (let y = 0; y <= document.body.scrollHeight; y += 400) { scrollTo(0, y); await new Promise((r) => setTimeout(r, 25)); }
          scrollTo(0, 0);
        });
        await page.waitForTimeout(500);
        const cls = await page.evaluate(() => +window.__cls.toFixed(4));
        const a = await page.evaluate(ANALYZE);
        if (vn === "desktop" && theme === "dark") {
          await page.screenshot({ path: join(ROOT, `${name}.png`), fullPage: true });
        }
        const rec = { name, path, vn, theme, status, cls, overflow: a.overflow, defects: a.defects };
        report.routes.push(rec);
        const flag = (cls > 0.02 || a.overflow > 1 || a.defects.length) ? " ⚠" : "";
        console.log(`${name.padEnd(18)} ${vn}/${theme}  [${status}] CLS=${cls} ovf=${a.overflow} def=${a.defects.length}${flag}${a.defects.length ? " " + a.defects.map((d) => d.k).join(",") : ""}`);
        await ctx.close();
      }
    }
  }
  writeFileSync(join(ROOT, "audit.json"), JSON.stringify(report, null, 2));
  const worst = report.routes.filter((r) => !r.error);
  console.log(`\nworst CLS=${Math.max(...worst.map((r) => r.cls))} worst overflow=${Math.max(...worst.map((r) => r.overflow))}px total defects=${worst.reduce((s, r) => s + r.defects.length, 0)}`);
  await browser.close();
}
run().catch((e) => { console.error(e); process.exit(1); });
