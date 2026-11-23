import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

test("toolbar glass styling does not move absolute corner controls into the grid", () => {
  const sheet = readFileSync(new URL("../../../static/themes/liquid-glass/styles/fancy-toolbar.scss", import.meta.url), "utf8");
  assert.doesNotMatch(sheet, /\.ft-bar\s*>\s*\*\s*\{/);
  assert.match(sheet, /\.ft-bar\s*>\s*\.ft-bar-container\s*\{\s*position:\s*relative;/);
});
