import assert from "node:assert/strict";
import fs from "node:fs/promises";
import { readFileSync } from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import yaml from "js-yaml";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.MACUI_PLAYWRIGHT_MODULE || "playwright");
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const dist = path.join(root, "dist");
const output = process.env.MACUI_QA_OUTPUT || path.join(root, "target/qa/settings-categories");
await fs.mkdir(output, { recursive: true });

function resource(file) {
  const schema = yaml.DEFAULT_SCHEMA.extend([
    new yaml.Type("!extend", { kind: "scalar", construct: (name) => resource(path.resolve(path.dirname(file), name)) }),
    new yaml.Type("!include", {
      kind: "scalar",
      construct: (name) => readFileSync(path.resolve(path.dirname(file), name), "utf8"),
    }),
  ]);
  return yaml.load(readFileSync(file, "utf8"), { schema });
}
function normalizeSettings(definitions = []) {
  const types = {
    switch: "Switch",
    select: "Select",
    text: "InputText",
    "input-text": "InputText",
    number: "InputNumber",
    "input-number": "InputNumber",
    range: "Range",
    color: "Color",
    font: "Font",
  };
  return definitions.map((definition) => {
    if (definition.group) {
      return { group: { ...definition.group, items: normalizeSettings(definition.group.items) } };
    }
    return {
      ...definition,
      type: types[definition.type] || definition.type,
      subtype: definition.subtype ? definition.subtype[0].toUpperCase() + definition.subtype.slice(1) : "List",
    };
  });
}
const widgets = [
  "desktop-shell",
  "weg",
  "toolbar",
  "apps-menu",
  "window-manager",
  "task-switcher",
  "flyouts",
  "wallpaper-manager",
].map((folder) => {
  const widget = resource(path.join(root, "src/static/widgets", folder, "metadata.yml"));
  return {
    plugins: [],
    shortcuts: [],
    hidden: false,
    ...widget,
    settings: normalizeSettings(widget.settings),
    metadata: { ...widget.metadata, bundled: true },
  };
});
const byWidget = Object.fromEntries(widgets.map((widget) => [widget.id, { enabled: true }]));
Object.assign(byWidget["@macui/weg"], {
  size: 48,
  position: "Bottom",
  mode: "MinContent",
  margin: 8,
  padding: 8,
  spaceBetweenItems: 8,
  hideMode: "Never",
  windowAnimation: "Genie",
  middleClickAction: "OpenNewInstance",
  delayToHide: 200,
  delayToShow: 100,
});
Object.assign(byWidget["@macui/fancy-toolbar"], {
  itemSize: 24,
  margin: 0,
  padding: 4,
  position: "Top",
  hideMode: "Never",
  delayToHide: 200,
  delayToShow: 100,
});
Object.assign(byWidget["@macui/window-manager"], {
  border: { enabled: false, width: 2, offset: 0 },
  floating: { width: 800, height: 600 },
  animations: { enabled: true, durationMs: 200, easeFunction: "Linear" },
  workspaceGap: 8,
  workspacePadding: 8,
  workspaceMargin: { left: 0, top: 0, right: 0, bottom: 0 },
  resizeDelta: 10,
  dragBehavior: "Sort",
  stackBarVisibility: "Always",
});
Object.assign(byWidget["@macui/wallpaper-manager"], {
  interval: 3600,
  randomize: false,
  multimonitorBehaviour: "PerMonitor",
  coveragePauseThreshold: .9,
});
const data = {
  language: "zh-CN",
  byWidget,
  byApp: [],
  byWallpaper: {},
  monitorsV3: {},
  wallpaperCollections: [],
  activeIconPacks: [],
  devTools: false,
  dateFormat: "YYYY-MM-DD",
  startOfWeek: "Monday",
  pollingInterval: 1,
  hardwareAcceleration: true,
  performanceMode: { default: "Disabled", onBattery: "Minimal", onEnergySaver: "Extreme" },
  shortcuts: { enabled: true, shortcuts: {} },
};

const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1000, height: 740 } });
  const errors = [];
  page.on("pageerror", (error) => {
    errors.push(error.message);
    console.error(error.message);
  });
  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    if (url.origin !== "http://settings.localhost") return route.abort();
    const file = path.resolve(dist, "." + decodeURIComponent(url.pathname));
    if (!file.startsWith(dist + path.sep)) return route.abort();
    if (url.pathname === "/vanilla/entry-point/index.js") {
      return route.fulfill({ contentType: "application/javascript", body: "import '/react/settings/index.js'" });
    }
    try {
      await route.fulfill({
        body: await fs.readFile(file),
        contentType: {
          ".js": "application/javascript",
          ".css": "text/css",
          ".html": "text/html",
          ".svg": "image/svg+xml",
          ".json": "application/json",
          ".png": "image/png",
        }[path.extname(file)] || "application/octet-stream",
      });
    } catch {
      await route.fulfill({ status: 404, body: "" });
    }
  });
  await page.addInitScript(({ data, widgets }) => {
    const persisted = localStorage.getItem("qa-settings");
    if (persisted) Object.assign(data, JSON.parse(persisted));
    window.exitCalls = [];
    const noop = async () => {};
    window.__MACUI_WIDGET = { id: "@macui/settings", settings: [] };
    window.__MACUI_WIDGET_INSTANCE = {
      id: "@macui/settings",
      decoded: {},
      init: noop,
      ready: noop,
      show: noop,
      onTrigger: noop,
      window: new Proxy({}, { get: () => noop }),
      webview: { listen: noop },
    };
    let nextId = 0;
    const callbacks = new Map();
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    const listeners = new Map();
    window.desktopControls = {
      arrangeMode: "category",
      iconSize: 64,
      snapToGrid: true,
      showGrid: false,
      layoutLocked: false,
      glassTransparency: 88,
      glassBlur: 35,
      snapBoxes: true,
      horizontalSpacing: 8,
      verticalSpacing: 10,
    };
    window.desktopPatches = [];
    const publish = (name, payload) => {
      for (const [id, listener] of listeners) {
        if (listener.event === name) callbacks.get(listener.handler)?.({ event: name, id, payload });
      }
    };
    window.publishDesktopControls = (patch) => {
      Object.assign(window.desktopControls, patch);
      publish("macui-desktop-controls-changed", structuredClone(window.desktopControls));
    };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "settings" }, currentWebview: { label: "settings" } },
      transformCallback: (callback) => {
        const id = ++nextId;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback() {},
      convertFileSrc: (p) => p,
      invoke: (command, args) =>
        Promise.resolve().then(() => {
          if (command === "plugin:event|listen") {
            const id = ++nextId;
            listeners.set(id, args);
            return id;
          }
          if (command === "plugin:event|unlisten") {
            listeners.delete(args.eventId);
            return;
          }
          if (command === "plugin:event|emit" && args.event === "macui-desktop-controls-request") {
            const request = args.payload;
            if (request.patch) {
              window.desktopPatches.push(request.patch);
              Object.assign(window.desktopControls, request.patch);
            }
            publish("macui-desktop-controls-response", {
              id: request.id,
              controls: structuredClone(window.desktopControls),
            });
            return;
          }
          if (command === "plugin:process|exit") {
            window.exitCalls.push(args);
            return;
          }
          if (command === "set_frosted_regions") {
            window.lastFrost = structuredClone(args);
            return;
          }
          if (command === "state_write_settings") {
            if (window.failSave) throw new Error("Test storage is unavailable");
            window.lastSaved = structuredClone(args.settings);
            Object.assign(data, structuredClone(args.settings));
            localStorage.setItem("qa-settings", JSON.stringify(args.settings));
            return;
          }
          if (["state_get_default_settings", "state_get_settings"].includes(command)) return structuredClone(data);
          if (command === "state_get_widgets") return widgets;
          if (command === "get_system_colors") {
            return Object.fromEntries(
              [
                "accent",
                "accent_light",
                "accent_lighter",
                "accent_lightest",
                "accent_dark",
                "accent_darker",
                "accent_darkest",
              ].map((key) => [key, { r: 23, g: 92, b: 211, a: 255 }]),
            );
          }
          if (command === "state_get_default_monitor_settings") return { byWidget: {}, byWorkspace: {} };
          if (command === "get_connected_monitors") {
            return [{
              id: "display-1",
              name: "Display 1",
              isPrimary: true,
              rect: { left: 0, top: 0, right: 1920, bottom: 1080 },
            }];
          }
          if (command === "get_virtual_desktops") return { monitors: {} };
          if (command === "get_auto_start_status" || command === "is_touch_primary") return false;
          if (command === "read_data_file") return JSON.stringify({ welcomeShown: true, alreadyReviewed: true });
          if (command.startsWith("plugin:event|")) return ++nextId;
          if (command.startsWith("plugin:app|")) return "4.1.1";
          if (command.startsWith("plugin:path|")) return "";
          return [];
        }),
    };
    document.addEventListener("DOMContentLoaded", () => {
      document.documentElement.dataset.widgetId = "@macui/settings";
    });
  }, { data, widgets });
  const navigate = async (route) => {
    await page.evaluate((value) => {
      location.hash = value;
    }, route);
    await page.waitForTimeout(250);
  };
  await page.goto("http://settings.localhost/react/settings/index.html#/");
  await page.getByRole("heading", { name: "设置", exact: true }).waitFor();
  await page.locator("#splashscreen").waitFor({ state: "hidden" });
  const welcome = page.getByRole("button", { name: "开始使用", exact: true });
  if (await welcome.isVisible()) await welcome.click();
  const categories = ["desktop", "dock", "toolbar", "windows", "appearance", "system"];
  for (const name of ["最小化", "最大化或还原", "关闭"]) {
    const control = await page.getByRole("button", { name, exact: true }).evaluate((el) => {
      const button = el.getBoundingClientRect();
      const icon = el.querySelector("svg").getBoundingClientRect();
      return {
        width: button.width,
        height: button.height,
        radius: getComputedStyle(el).borderTopLeftRadius,
        horizontalOffset: icon.x + icon.width / 2 - button.x - button.width / 2,
        verticalOffset: icon.y + icon.height / 2 - button.y - button.height / 2,
      };
    });
    assert.equal(control.width, control.height, name);
    assert.equal(control.radius, "50%", name);
    assert.ok(Math.abs(control.horizontalOffset) < 0.5, name);
    assert.ok(Math.abs(control.verticalOffset) < 0.5, name);
  }
  assert.ok(
    await page.getByRole("button", { name: "退出 MAC UI", exact: true }).evaluate((el) =>
      getComputedStyle(el).backgroundImage.includes("255, 224, 229")
    ),
  );
  for (const width of [1568, 1000, 720]) {
    await page.setViewportSize({ width, height: 740 });
    await navigate("/");
    assert.equal(await page.locator("main a").count(), 7);
    await page.screenshot({ path: path.join(output, `home-${width}.png`) });
    for (const category of categories) {
      await page.locator(`main a[href="#/${category}"]`).click();
      await page.locator("main h1").waitFor();
      assert.equal(await page.locator("main > div").evaluate((el) => getComputedStyle(el).opacity), "1");
      assert.equal(await page.locator("main nav, main [role=tablist], main details").count(), 0);
      assert.ok(await page.locator("main [data-setting]").count() > 0, `${category} has controls`);
      assert.equal(
        await page.locator("[data-settings-flow]").evaluate((el) => getComputedStyle(el).columnCount),
        "2",
        `${category} uses two columns at ${width}px`,
      );
      const splitCards = await page.locator("[data-settings-flow] [data-settings-card]").evaluateAll((cards) =>
        cards.filter((card) => card.getClientRects().length > 1).length
      );
      assert.equal(splitCards, 0, `${category} keeps cards together at ${width}px`);
      if (category === "appearance") {
        const geometry = await page.evaluate(() => {
          const main = document.querySelector("main").getBoundingClientRect();
          const flow = document.querySelector("[data-settings-flow]").getBoundingClientRect();
          const wallpaper = document.querySelector('[data-section="wallpaper-manager"] [data-feature-header]')
            .getBoundingClientRect();
          const glass = document.querySelector('[data-section="glass"] h2').getBoundingClientRect();
          const dock = document.querySelector('[data-mac-material="dock"]').closest("[data-settings-card]")
            .getBoundingClientRect();
          const colors = document.querySelector('[data-section="colors"] [data-settings-card]')
            .getBoundingClientRect();
          return {
            mainWidth: main.width,
            flowWidth: flow.width,
            wallpaperTop: wallpaper.top,
            wallpaperLeft: wallpaper.left,
            glassTop: glass.top,
            glassLeft: glass.left,
            dockTop: dock.top,
            colorsBottom: colors.bottom,
          };
        });
        assert.ok(geometry.flowWidth >= geometry.mainWidth * .88, `appearance uses available width at ${width}px`);
        assert.ok(
          Math.abs(geometry.wallpaperTop - geometry.glassTop) < 36,
          "wallpaper controls start beside appearance controls",
        );
        assert.ok(geometry.wallpaperLeft > geometry.glassLeft + 250, "wallpaper has its own column");
        assert.ok(
          geometry.dockTop - geometry.colorsBottom <= 20,
          "Dock controls follow the color card without a blank row",
        );
      }
      const dimensions = await page.locator("main").evaluate((el) => ({
        content: el.scrollWidth,
        width: el.clientWidth,
        bottom: el.getBoundingClientRect().bottom,
      }));
      if (dimensions.content > dimensions.width + 2) {
        await page.screenshot({ path: path.join(output, `${category}-overflow-${width}.png`) });
        console.log(
          await page.locator("main *").evaluateAll((elements) =>
            elements.filter((el) => el.scrollWidth > el.clientWidth + 10 && el.clientWidth > 0).map((el) => ({
              tag: el.tagName,
              class: el.className,
              width: el.clientWidth,
              content: el.scrollWidth,
            })).slice(0, 18)
          ),
        );
      }
      assert.ok(dimensions.content <= dimensions.width + 2, `${category} fits ${width}px`);
      assert.equal(dimensions.bottom, 740);
      await page.screenshot({ path: path.join(output, `${category}-${width}.png`), animations: "disabled" });
      await page.getByRole("link", { name: "返回首页" }).click();
    }
  }
  await page.setViewportSize({ width: 1568, height: 1290 });
  await navigate("/appearance");
  await page.screenshot({ path: path.join(output, "appearance-tall.png"), animations: "disabled" });
  await page.setViewportSize({ width: 1000, height: 740 });
  await navigate("/appearance?section=wallpapers");
  const libraryHeading = page.locator("#section-wallpapers");
  assert.ok(await libraryHeading.isVisible());
  const libraryBox = await libraryHeading.boundingBox();
  assert.ok(
    libraryBox.y >= 66 && libraryBox.y + libraryBox.height < 740,
    "section links scroll to the visible heading",
  );
  assert.ok(await page.locator("main").evaluate((element) => element.scrollTop > 0));
  await navigate("/desktop");
  const transparency = page.getByRole("slider", { name: "玻璃透明度", exact: true });
  await transparency.focus();
  await page.keyboard.press("End");
  assert.deepEqual(await page.evaluate(() => window.desktopPatches.at(-1)), { glassTransparency: 100 });
  const frost = page.getByRole("slider", { name: "磨砂强度", exact: true });
  await frost.focus();
  await page.keyboard.press("Home");
  assert.deepEqual(await page.evaluate(() => window.desktopPatches.at(-1)), { glassBlur: 0 });
  const edgeSwitch = page.getByRole("switch", { name: "整理盒边缘吸附", exact: true });
  await edgeSwitch.click();
  assert.deepEqual(await page.evaluate(() => window.desktopPatches.at(-1)), { snapBoxes: false });
  const horizontalSpacing = page.getByRole("spinbutton", { name: "水平间距", exact: true });
  await horizontalSpacing.focus();
  await page.keyboard.press("ArrowUp");
  assert.deepEqual(await page.evaluate(() => window.desktopPatches.at(-1)), { horizontalSpacing: 9 });
  const gridSwitch = page.getByRole("switch", { name: "显示网格", exact: true });
  await gridSwitch.click();
  assert.deepEqual(await page.evaluate(() => window.desktopPatches.at(-1)), { showGrid: true });
  await page.evaluate(() => window.publishDesktopControls({ showGrid: false, layoutLocked: true }));
  assert.equal(await gridSwitch.getAttribute("aria-checked"), "false");
  assert.equal(
    await page.getByRole("switch", { name: "锁定整理框", exact: true }).getAttribute("aria-checked"),
    "true",
  );
  await navigate("/widget?id=%40macui%2Fapps-menu");
  assert.ok(page.url().endsWith("#/dock?section=apps-menu"));
  const desktopSwitch = page.locator('[data-section="apps-menu"] [role="switch"]').first();
  const before = await desktopSwitch.getAttribute("aria-checked");
  await desktopSwitch.click();
  await page.getByRole("link", { name: "返回首页" }).click();
  await page.locator('main a[href="#/dock"]').click();
  assert.notEqual(await desktopSwitch.getAttribute("aria-checked"), before);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/apps-menu"].enabled), before !== "true");
  await desktopSwitch.click();
  await page.getByRole("button", { name: "撤销更改", exact: true }).click();
  assert.notEqual(await desktopSwitch.getAttribute("aria-checked"), before);
  const range = page.getByRole("slider", { name: "Dock 整体大小", exact: true });
  await range.focus();
  await page.keyboard.press("End");
  assert.equal(await range.inputValue(), "128");
  const large = await page.locator("[data-dock-preview] > span").first().evaluate((el) =>
    el.getBoundingClientRect().width
  );
  await page.keyboard.press("Home");
  const small = await page.locator("[data-dock-preview] > span").first().evaluate((el) =>
    el.getBoundingClientRect().width
  );
  assert.ok(large > small * 3, "size preview follows the range input");
  await page.getByRole("button", { name: "右侧", exact: true }).click();
  assert.equal(await page.locator("[data-dock-preview]").getAttribute("data-side"), "Right");
  await page.getByRole("button", { name: "保存", exact: true }).click();
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/weg"].position), "Right");
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/weg"].size), 16);
  await page.getByRole("button", { name: "底部", exact: true }).click();
  await page.evaluate(() => {
    window.failSave = true;
  });
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await page.getByRole("dialog").waitFor();
  assert.ok(await page.getByRole("dialog").innerText().then((text) => text.includes("设置保存失败")));
  await page.getByRole("dialog").getByRole("button").click();
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  await page.evaluate(() => {
    window.failSave = false;
  });
  await page.getByRole("button", { name: "撤销更改", exact: true }).click();
  assert.equal(await page.locator("[data-dock-preview]").getAttribute("data-side"), "Right");
  for (const viewport of [{ width: 720, height: 520 }, { width: 1440, height: 900 }]) {
    await page.setViewportSize(viewport);
    const bounds = await page.locator("main").evaluate((element) => ({
      bottom: element.getBoundingClientRect().bottom,
      width: element.clientWidth,
      content: element.scrollWidth,
    }));
    assert.equal(bounds.bottom, viewport.height);
    assert.ok(bounds.content <= bounds.width + 2);
  }
  await page.setViewportSize({ width: 1000, height: 740 });
  await navigate("/toolbar?section=flyouts");
  const messages = page.getByRole("switch", { name: "新消息提醒", exact: true });
  await messages.waitFor();
  assert.equal(await messages.getAttribute("aria-checked"), "true");
  assert.equal(await page.getByText("到货通知", { exact: true }).count(), 0);
  assert.equal(await page.getByText("体积变化", { exact: true }).count(), 0);
  assert.ok(await page.getByText("收到微信、企业微信等应用的新消息时，显示提醒弹窗。", { exact: true }).isVisible());
  await messages.click();
  await page.getByRole("button", { name: "退出 MAC UI", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "继续编辑", exact: true }).click();
  assert.equal(await messages.getAttribute("aria-checked"), "false");
  assert.equal(await page.evaluate(() => window.exitCalls.length), 0);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/flyouts"].showNotifications), false);
  await page.reload();
  await page.locator("#splashscreen").waitFor({ state: "hidden" });
  assert.equal(await messages.getAttribute("aria-checked"), "false");
  await messages.click();
  await page.getByRole("button", { name: "退出 MAC UI", exact: true }).click();
  await page.getByRole("dialog").getByRole("button", { name: "放弃更改并退出", exact: true }).click();
  await page.waitForFunction(() => window.exitCalls.length === 1);
  assert.equal(await messages.getAttribute("aria-checked"), "false");
  assert.equal(await page.evaluate(() => window.exitCalls[0].code), 0);
  await page.getByRole("button", { name: "退出 MAC UI", exact: true }).click();
  await page.waitForFunction(() => window.exitCalls.length === 2);
  assert.equal(await page.getByRole("dialog").count(), 0);
  await page.waitForFunction(() => window.lastFrost?.rects?.length === 1);
  assert.deepEqual(await page.evaluate(() => window.lastFrost.rects[0]), [0, 0, 1000, 740, 26]);
  assert.equal(await page.locator("html").getAttribute("data-native-frosted"), "true");
  await page.screenshot({ path: path.join(output, "notifications-glass-1000.png"), animations: "disabled" });
  await navigate("/appearance");
  await page.getByRole("button", { name: "增强对比", exact: true }).click();
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/settings"].colorScheme), "dark");
  await page.getByRole("button", { name: "原版玻璃", exact: true }).click();
  assert.equal(await page.evaluate(() => window.lastSaved.byWidget["@macui/settings"].colorScheme), "light");
  await page.keyboard.press("Control+k");
  await page.getByRole("dialog").getByRole("textbox").fill("通知");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Enter");
  assert.ok(page.url().includes("/toolbar"));
  await page.keyboard.press("Control+k");
  await page.getByRole("dialog").waitFor();
  await page.keyboard.press("Escape");
  await page.getByRole("dialog").waitFor({ state: "hidden" });
  for (
    const [route, expected] of [
      ["/general", "/system?section=general"],
      ["/shortcuts", "/windows?section=shortcuts"],
      ["/monitors", "/system?section=monitors"],
      ["/specific_apps", "/system?section=applications"],
      ["/wallpapers", "/appearance?section=wallpapers"],
    ]
  ) {
    await navigate(route);
    assert.ok(page.url().endsWith("#" + expected), route);
  }
  await page.emulateMedia({ reducedMotion: "reduce" });
  await navigate("/");
  assert.equal(await page.locator("main > div").evaluate((el) => getComputedStyle(el).animationName), "none");
  assert.deepEqual(errors, []);
  console.log(
    "PASS six categories, compact layout at 720/1000/1568px, category grouping, legacy entries, drafts/save/revert, save failure, Dock preview, glass modes, keyboard search, reduced motion and no runtime exceptions.",
  );
} finally {
  await browser.close();
}
