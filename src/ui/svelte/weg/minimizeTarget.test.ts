import assert from "node:assert/strict";
import { test } from "node:test";
import { minimizeTarget, readMinimizeTarget } from "./minimizeTarget.ts";

test("Genie target uses physical webview origin and fractional DPI", () => {
  assert.deepEqual(minimizeTarget({ left: 120, top: 800, width: 48, height: 48 }, { x: -2560, y: 60 }, 1.5),
    { left: -2380, top: 1260, right: -2308, bottom: 1332 });
});

test("invalid or zero-size targets fall back to the native minimize path", () => {
  const icon = { left: 10, top: 20, width: 48, height: 48 };
  assert.equal(minimizeTarget(icon, { x: 0, y: 0 }, 0), null);
  assert.equal(minimizeTarget({ ...icon, width: 0 }, { x: 0, y: 0 }, 1), null);
  assert.equal(minimizeTarget(icon, { x: NaN, y: 0 }, 1), null);
});

test("multi-window preview accepts a physical icon target and rejects malformed IPC bounds", () => {
  const target = { left: -2380, top: 1260, right: -2308, bottom: 1332 };
  assert.deepEqual(readMinimizeTarget(target), target);
  for (const invalid of [null, {}, { ...target, left: "-2380" }, { ...target, right: Infinity },
    { ...target, right: target.left }, { ...target, right: target.left + 513 }]) {
    assert.equal(readMinimizeTarget(invalid), null);
  }
});
