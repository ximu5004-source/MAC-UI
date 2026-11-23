import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = (name: string) => readFileSync(new URL(name, import.meta.url), "utf8");
const app = source("./App.svelte");
const css = source("./launchpad.css");
const folder = source("./components/FolderModal.svelte");

test("account and power sheets dismiss Launchpad before triggering their own windows", () => {
  const footer = source("./components/layout/StartMenuFooter.svelte");
  for (const action of ["openUserMenu", "openPowerMenu"]) {
    assert.match(footer, new RegExp(`async function ${action}\\(\\) \\{[\\s\\S]*?await Widget\\.self\\.hide\\(\\);[\\s\\S]*?await invoke\\(MacUiCommand\\.TriggerWidget`));
  }
});

test("Launchpad presents one macOS-style grid without Windows partitions or Quick Access", () => {
  assert.doesNotMatch(app, /QuickAccess|launchpad-section-heading|<h2/);
  assert.match(app, /<StartMenuBody\s*\/>/);
  assert.match(app, /launchpad-view-switch/);
  assert.match(app, /launchpad-actions/);
  assert.match(css, /\.launchpad \.launchpad-applications\s*\{[^}]*border: 0;[^}]*background: transparent;/s);
});

test("folder opening hides the mounted root app grid instead of painting overlapping captions", () => {
  assert.match(app, /data-folder-open=\{!!openFolder\}/);
  assert.match(app, /<section[^>]*aria-hidden=\{!!openFolder\} inert=\{!!openFolder\}/);
  assert.match(css, /\[data-folder-open=true\] \.launchpad-applications\s*\{[^}]*visibility: hidden;[^}]*pointer-events: none;/s);
  assert.match(folder, /pageGeometry\(viewportWidth, viewportHeight \+ 40, 64\)/);
  assert.match(folder, /bind:clientWidth=\{viewportWidth\} bind:clientHeight=\{viewportHeight\}/);
});

test("application labels are legible full-opacity medium 16px text with no glow or transform", () => {
  assert.match(css, /\.apps-menu\.launchpad \.folder \.folder-name\s*\{[^}]*font-size: 16px;[^}]*font-weight: 500;[^}]*line-height: 20px;[^}]*opacity: 1;[^}]*text-shadow: none;[^}]*filter: none;[^}]*transform: none;/s);
  assert.match(css, /color-scheme: inherit;/);
  assert.match(source("./components/AppItem.svelte"), /appearance="plain"/);
  assert.match(source("./components/FolderItem.svelte"), /appearance="plain"/);
  assert.doesNotMatch(css, /drop-shadow|text-rendering|font-smoothing/);
});

test("autofocus search keeps a neutral boundary while deliberate keyboard navigation has a focus indicator", () => {
  assert.match(css, /\.launchpad \.launchpad-search:focus-within\s*\{[^}]*border-color: var\(--lp-border\);[^}]*outline: 0;/s);
  assert.match(css, /\[data-keyboard-focus=true\] \.launchpad-search:has\(input:focus-visible\)\s*\{[^}]*outline: 2px solid var\(--lp-fg\);/s);
  assert.match(app, /if \(event\.key === "Tab"\) keyboardFocus = true;/);
});

test("fullscreen window stays edge-to-edge while its readable composition is centered and bounded", () => {
  const fullscreen = css.match(/\.apps-menu\.launchpad\[data-fullscreen=true\]\s*\{([^}]+)\}/)?.[1] || "";
  assert.match(fullscreen, /width: 100%;/);
  assert.match(fullscreen, /height: 100%;/);
  assert.match(fullscreen, /margin: 0;/);
  assert.match(fullscreen, /border-radius: 0;/);
  assert.doesNotMatch(fullscreen, /1440px|calc\(100% -/);
  assert.match(app, /class="launchpad-content"/);
  const content = css.match(/\.apps-menu\.launchpad\[data-fullscreen=true\] \.launchpad-content\s*\{([^}]+)\}/)?.[1] || "";
  assert.match(content, /width: min\(1280px, 100%\);/);
  assert.match(content, /height: min\(900px, 100%\);/);
  assert.match(content, /background: light-dark\(/);
  assert.match(css, /grid-template-rows: repeat\(var\(--launchpad-rows\), var\(--lp-cell-height\)\);/);
});

test("clear favorites is visible outside selection mode and requires an accessible confirmation", () => {
  assert.ok(app.indexOf('class="launchpad-clear"') < app.indexOf("{#if globalState.choosingFavorites}"));
  assert.match(app, /<ClearFavoritesDialog onClose=\{closeClearDialog\}/);
  const dialog = source("./components/ClearFavoritesDialog.svelte");
  assert.match(dialog, /role="alertdialog" aria-labelledby="launchpad-clear-title" aria-describedby="launchpad-clear-description"/);
  assert.match(dialog, /cancelButton\?\.focus\(\)/);
  assert.match(dialog, /if \(await globalState\.clearFavorites\(\)\)/);
  assert.match(dialog, /globalState\.chooseFavorites\(\)/);
});

test("launchpad searches only Start Menu catalog and never bootstraps personal-file indexing", () => {
  const state = source("./state/mod.svelte.ts");
  const search = source("./state/search.svelte.ts");
  assert.doesNotMatch(state + search, /from ["']\.\/knownFolders|GetUserFolderContent|GetNativeStartMenu/);
  assert.match(search, /globalState\.allItems/);
  assert.doesNotMatch(state, /importStartMenu\(/);
  assert.doesNotMatch(app, /query\.files|query\.web|importWindowsStartMenu/);
  assert.match(app, /launchpad\.favorites/);
  assert.match(app, /chooseFavorites\(\)/);
});
