import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = (name: string) => readFileSync(new URL(name, import.meta.url), "utf8");
const app = source("./app.svelte");
const entry = source("./index.ts");
const css = source("./calendar.css");

test("calendar opts into the shared neutral frost with a 28px rounded surface", () => {
  assert.match(app, /slu-std-popover mac-panel mac-frosted-surface calendar-popup/);
  assert.match(entry, /import "libs\/ui\/svelte\/styles\/mac-panels\.css"/);
  assert.match(entry, /await configureMacPanel\(widget\)/);
  assert.match(entry, /trackFrostedSurfaces\("\.calendar-popup\.mac-frosted-surface"\)/);
  assert.doesNotMatch(entry, /trackFrostedSurfaces\([^)]*,\s*true/);
  assert.match(css, /border-radius: 28px/);
  assert.match(css, /color-scheme: light dark/);
  assert.doesNotMatch(css, /drop-shadow|backdrop-filter|linear-gradient/);
});

test("calendar navigation and date selection are semantic keyboard-operable buttons", () => {
  assert.doesNotMatch(app, /role="button"|onkeydown=\{\(e\)/);
  assert.match(app, /aria-label=\{\$t\(globalState\.viewMode === "month" \? "previous_month" : "previous_year"\)\}/);
  assert.match(app, /aria-label=\{\$t\(globalState\.viewMode === "month" \? "next_month" : "next_year"\)\}/);
  assert.match(app, /aria-label=\{\$t\("today"\)\}/);
  assert.match(app, /aria-label=\{day\.format\("LL"\)\}/);
  assert.match(app, /aria-pressed=\{isSelected\}/);
  assert.match(app, /aria-current=\{isToday \? "date" : undefined\}/);
  assert.match(app, /onwheel=\{handleWheel\}/);
  assert.match(css, /:focus-visible\s*\{[^}]*outline: 2px solid var\(--mac-accent\)/s);
});

test("calendar text and glyphs retain clear typography without glow or shadow", () => {
  assert.match(css, /box-shadow: none;\s*text-shadow: none;\s*filter: none;/);
  assert.match(css, /\.calendar-cell\s*\{[^}]*font-size: 14px;[^}]*font-weight: 500;/s);
  assert.match(css, /\.calendar-weekday\s*\{[^}]*opacity: 1;/s);
  assert.match(css, /@media \(forced-colors: active\)/);
});
