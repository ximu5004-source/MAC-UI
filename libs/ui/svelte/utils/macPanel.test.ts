import { test } from "node:test";
import assert from "node:assert/strict";
import { macPanelBounds } from "./macPanel";

test("popup bounds use monitor work area, not auto-sized webview height", () => {
  assert.deepEqual(macPanelBounds(1920, 1040), { width: 1900, height: 680 });
  assert.deepEqual(macPanelBounds(1920, 1040, 2), { width: 940, height: 488 });
  assert.deepEqual(macPanelBounds(640, 480), { width: 620, height: 448 });
});
test("tiny work areas and invalid DPI never produce negative or NaN bounds", () => {
  assert.deepEqual(macPanelBounds(10, 10), { width: 1, height: 1 });
  assert.deepEqual(macPanelBounds(0, NaN, 0), { width: 400, height: 680 });
});
