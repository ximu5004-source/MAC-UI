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
    assert.match(footer, new RegExp(`async function ${action}\\(\\) \\{[\\s\\S]*?await Widget\\.self\\.hide\\(\\);[\\s\\S]*?await invoke\\(SeelenCommand\\.TriggerWidget`));
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

test("application labels are crisp full-opacity medium 14px text with no glow or transform", () => {
  assert.match(css, /\.apps-menu\.launchpad \.folder \.folder-name\s*\{[^}]*font-size: 14px;[^}]*font-weight: 500;[^}]*line-height: 20px;[^}]*opacity: 1;[^}]*text-shadow: none;[^}]*filter: none;[^}]*transform: none;/s);
  assert.match(css, /color-scheme: light dark;/);
  assert.match(source("./components/AppItem.svelte"), /appearance="plain"/);
  assert.match(source("./components/FolderItem.svelte"), /appearance="plain"/);
  assert.doesNotMatch(css, /drop-shadow|text-rendering|font-smoothing/);
});

test("autofocus search keeps a neutral boundary while deliberate keyboard navigation has a focus indicator", () => {
  assert.match(css, /\.launchpad \.launchpad-search:focus-within\s*\{[^}]*border-color: var\(--lp-border\);[^}]*outline: 0;/s);
  assert.match(css, /\[data-keyboard-focus=true\] \.launchpad-search:has\(input:focus-visible\)\s*\{[^}]*outline: 2px solid var\(--lp-fg\);/s);
  assert.match(app, /if \(event\.key === "Tab"\) keyboardFocus = true;/);
});
