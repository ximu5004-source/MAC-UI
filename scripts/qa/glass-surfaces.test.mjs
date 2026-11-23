import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.MACUI_PLAYWRIGHT_MODULE || "playwright");
const read = (file) => fs.readFile(file, "utf8");
const shared = (await read("src/static/themes/default/shared/index.scss"))
  .replace(/^\s*\/\/.*$/gm, "").replace(/^@import.*$/gm, "");
const material = await read("libs/ui/styles/frosted-material.css");
const theme = await read("src/static/themes/default/styles/flyouts.css");
const flyout = await read("src/ui/svelte/flyouts/frosted-flyouts.css");
const menu = await read("src/ui/svelte/context-menu/glass-menu.css");
const menuTheme = await read("src/static/themes/default/styles/context-menu.scss");
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 420, height: 300 } });
  for (const dark of [false, true]) {
    for (const themeLast of [false, true]) {
      for (const app of ["微信", "企业微信"]) {
        const styles = themeLast ? [material, flyout, shared, theme] : [shared, theme, material, flyout];
        await page.setContent(
          `<style>:root{--color-gray-50:#fff;--color-gray-25:#fff;--color-gray-900:#171717;color-scheme:${
            dark ? "dark" : "light"
          }}</style>
          ${styles.map((css) => `<style>${css}</style>`).join("")}
          <div id="root"><div class="flyout mac-frosted-surface" data-showing="true"><article class="notification communication-notification">
          <div class="notification-header">${app}</div><button class="communication-content">您有一条来自“${app}”的新消息</button>
          </article></div></div>`,
        );
        const background = await page.evaluate(() =>
          [
            document.documentElement,
            document.body,
            document.querySelector("#root"),
            document.querySelector(".notification"),
          ]
            .map((node) => ({
              background: getComputedStyle(node).backgroundColor,
              shadow: getComputedStyle(node).boxShadow,
            }))
        );
        for (const layer of background) assert.equal(layer.background, "rgba(0, 0, 0, 0)");
        assert.equal(background.at(-1).shadow, "none");
        assert.equal(await page.locator(".flyout").evaluate((el) => getComputedStyle(el).borderTopLeftRadius), "22px");
      }
    }
  }
  await page.setContent(`<style>${material}\n${menu}\n${shared}\n${menuTheme}</style>
    <div id="root"><div class="macui-std-popover context-menu mac-frosted-surface"><button class="menu-item">清空回收站</button><hr><button class="menu-item">移除模块</button></div></div>`);
  assert.equal(await page.locator("body").evaluate((el) => getComputedStyle(el).backgroundColor), "rgba(0, 0, 0, 0)");
  const surface = await page.locator(".context-menu").evaluate((el) => ({
    background: getComputedStyle(el).backgroundColor,
    radius: getComputedStyle(el).borderTopLeftRadius,
  }));
  assert.match(surface.background, /rgba\(.+0\.1\)/);
  assert.equal(surface.radius, "18px");
  console.log(
    "PASS notification transparency in both color schemes and stylesheet orders; glass context menu surface.",
  );
} finally {
  await browser.close();
}
