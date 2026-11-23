import assert from "node:assert/strict";
import fs from "node:fs/promises";
import path from "node:path";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.MACUI_PLAYWRIGHT_MODULE || "playwright");
const dist = path.resolve("dist");
const mock = await fs.readFile("scripts/qa/mac-panels-mock.js", "utf8");
const shared = (await fs.readFile("src/static/themes/default/shared/index.scss", "utf8"))
  .replace(/^\s*\/\/.*$/gm, "").replace(/^@import.*$/gm, "");
const theme = await fs.readFile("src/static/themes/default/styles/tray-menu.scss", "utf8");
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  for (const scale of [1, 1.25, 2]) {
    const page = await browser.newPage({ viewport: { width: 400, height: 240 }, deviceScaleFactor: scale });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.route("**/*", async (route) => {
      const url = new URL(route.request().url());
      if (url.origin !== "http://tray.localhost") return route.abort();
      if (url.pathname.endsWith("index.html")) {
        return route.fulfill({
          contentType: "text/html",
          body: `<html lang="zh-CN"><head><style>${shared}</style><link rel="stylesheet" href="./index.css"></head>
        <body><div id="fixture-status" style="display:none"></div><div id="root"></div><script>${mock}</script><script type="module" src="./index.js"></script></body></html>`,
        });
      }
      if (url.pathname.startsWith("/fixture-icon/")) {
        return route.fulfill({
          contentType: "image/svg+xml",
          body:
            '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><circle cx="12" cy="12" r="10" fill="#175cd3"/></svg>',
        });
      }
      const file = path.resolve(dist, "." + url.pathname);
      if (!file.startsWith(dist + path.sep)) return route.abort();
      try {
        const body = await fs.readFile(file);
        return route.fulfill({
          body,
          contentType: {
            ".js": "text/javascript",
            ".css": "text/css",
            ".yml": "text/plain",
            ".svg": "image/svg+xml",
          }[path.extname(file)] || "application/octet-stream",
        });
      } catch {
        return route.fulfill({ status: 404, body: "" });
      }
    });
    await page.goto("http://tray.localhost/svelte/system-tray/index.html");
    await page.locator(".system-tray-item").first().waitFor();
    assert.equal(await page.locator(".system-tray-item").count(), 13);
    assert.equal(await page.locator(".tray-header, .tray-refresh, .tray-count").count(), 0);
    const gap = await page.locator(".system-tray").evaluate((el) =>
      el.querySelector(".system-tray-item").getBoundingClientRect().top - el.getBoundingClientRect().top
    );
    assert.ok(gap < 25, `Tray has compact top padding (${gap}px)`);
    await page.addStyleTag({ content: theme });
    for (const width of [300, 360]) {
      await page.locator("#root").evaluate(
        (root, width) => root.style.setProperty("--tray-width", `${width}px`),
        width,
      );
      const tile = page.locator(".system-tray-item").nth(1);
      await tile.hover();
      await tile.focus();
      const geometry = await page.locator(".system-tray-item").evaluateAll((tiles) =>
        tiles.map((tile) => {
          const button = tile.getBoundingClientRect();
          const box = tile.querySelector(".system-tray-item-icon-box").getBoundingClientRect();
          const icon = tile.querySelector(".system-tray-item-icon").getBoundingClientRect();
          const offset = (rect) => ({
            x: rect.x + rect.width / 2 - button.x - button.width / 2,
            y: rect.y + rect.height / 2 - button.y - button.height / 2,
          });
          return {
            width: button.width,
            height: button.height,
            iconWidth: icon.width,
            iconHeight: icon.height,
            boxOffset: offset(box),
            iconOffset: offset(icon),
          };
        })
      );
      for (const tile of geometry) {
        assert.equal(tile.width, 44);
        assert.equal(tile.height, 44);
        assert.equal(tile.iconWidth, 20);
        assert.equal(tile.iconHeight, 20);
        for (const offset of [tile.boxOffset, tile.iconOffset]) {
          assert.ok(
            Math.abs(offset.x) <= .5 && Math.abs(offset.y) <= .5,
            `Centered tray at ${scale}x and ${width}px: ${JSON.stringify(tile)}`,
          );
        }
      }
    }
    await page.evaluate(() => {
      window.trayRefreshes = 0;
      const invoke = window.__TAURI_INTERNALS__.invoke;
      window.__TAURI_INTERNALS__.invoke = (command, args) => {
        if (command === "get_system_tray_icons") window.trayRefreshes++;
        return invoke(command, args);
      };
    });
    await page.keyboard.press("F5");
    await page.waitForFunction(() => window.trayRefreshes === 1);
    assert.deepEqual(errors, []);
    if (process.env.MACUI_QA_OUTPUT) {
      await fs.mkdir(process.env.MACUI_QA_OUTPUT, { recursive: true });
      await page.screenshot({ path: path.join(process.env.MACUI_QA_OUTPUT, `tray-centered-${scale}.png`) });
    }
    await page.close();
  }
  console.log(
    "PASS centered tray icons, square hover/focus areas, late theme styles, 100/125/200% display scaling, compact layout and keyboard refresh.",
  );
} finally {
  await browser.close();
}
