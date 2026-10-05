import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = (name: string) => readFileSync(new URL(name, import.meta.url), "utf8");
const app = source("./app.svelte");
const css = source("./power-menu.css");
const entry = source("./index.ts");

test("only the session sheet opts into shared native frost without wallpaper-based foreground", () => {
  assert.match(app, /class="power-sheet mac-frosted-surface"/);
  assert.match(entry, /import "libs\/ui\/styles\/frosted-material\.css"/);
  assert.match(entry, /trackFrostedSurfaces\("\.power-sheet\.mac-frosted-surface"\);/);
  assert.doesNotMatch(entry, /trackFrostedSurfaces\([^;]*,\s*true\)/);
  assert.match(css, /color-scheme: light dark;/);
  assert.match(css, /--power-ink: light-dark\(#000, #fff\);/);
});

test("sheet has 32px corners, neutral shared material, and no exterior shadow or glyph halo", () => {
  assert.match(css, /\.power-sheet\s*\{[^}]*border-radius: 32px;[^}]*background: var\(--mac-glass-surface\);[^}]*backdrop-filter: var\(--mac-glass-blur\);[^}]*border: 0;[^}]*box-shadow: none;/s);
  assert.match(css, /\.power-menu-overlay\.mac-power\s*\{[^}]*background: transparent;/s);
  assert.match(css, /\.power-action-icon\s*\{[^}]*background: transparent;[^}]*box-shadow: none;[^}]*filter: none;/s);
  assert.doesNotMatch(css, /drop-shadow|blur\(6px\)/);
  for (const declaration of css.matchAll(/text-shadow:\s*([^;]+);/g)) {
    assert.equal(declaration[1]?.trim(), "none");
  }
  assert.doesNotMatch(app, /<style>/);
});

test("session text is crisp and small-screen layout keeps labels and controls available", () => {
  assert.match(css, /\.power-action-label\s*\{[^}]*font-size: 14px;[^}]*line-height: 20px;/s);
  assert.match(css, /\.power-sheet\s*\{[^}]*overflow-x: hidden;[^}]*overflow-y: auto;/s);
  assert.match(css, /\.power-footer\s*\{[^}]*flex-wrap: wrap;/s);
  assert.match(css, /@media \(max-width: 480px\)/);
  assert.match(css, /@media \(prefers-reduced-motion: reduce\)/);
  assert.match(css, /\(forced-colors: active\)/);
});

test("destructive action confirmation gives Cancel initial keyboard focus", () => {
  assert.match(app, /if \(!needsPowerConfirmation\(option.key\)\)/);
  assert.match(app, /selected = option;\s*await tick\(\); cancelButton\?\.focus\(\)/);
  assert.match(app, /bind:this=\{cancelButton\} disabled=\{busy\}/);
  assert.match(css, /button:focus-visible\s*\{[^}]*outline: 2px solid var\(--power-ink\);/s);
});

test("reopening, duplicate actions, and held Escape cannot clear a pending request or accept a power action", () => {
  assert.match(app, /async function reset\(\)\s*\{\s*if \(busy\) return;/);
  assert.match(app, /async function execute\(option: Option\)\s*\{\s*if \(busy\) return;/);
  assert.match(app, /async function choose\(option: Option\)\s*\{\s*if \(busy\) return;/);
  assert.match(app, /async function back\(\)\s*\{\s*if \(busy\) return;/);
  assert.match(app, /if \(event.isComposing \|\| event.defaultPrevented\) return;/);
  assert.match(app, /if \(event.repeat\) return;/);
  assert.match(app, /if \(selected && !busy\) void back\(\); else dismiss\(\);/);
});

test("failed operations keep an alert and restore safe Cancel focus", () => {
  assert.match(app, /catch \(cause\)\s*\{\s*error = String\(cause\);/);
  assert.match(app, /if \(error\) \{ await tick\(\); cancelButton\?\.focus\(\); \}/);
  assert.match(app, /class="power-error" role="alert"/);
  assert.match(app, /if \(!busy\) void Widget.self.hide\(\)/);
  assert.match(app, /if \(!sheet.contains\(document.activeElement\)\)/);
});
