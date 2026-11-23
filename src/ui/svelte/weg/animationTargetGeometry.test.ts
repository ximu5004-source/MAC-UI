import assert from "node:assert/strict";
import { test } from "node:test";
import { isDockTranslation, restingDockIconTarget } from "./animationTargetGeometry.ts";

test("browser DOMRect prototype dimensions survive resting-target conversion", () => {
  class BrowserRect {
    get left() { return 1800; }
    get top() { return 1082; }
    get width() { return 50; }
    get height() { return 50; }
  }
  const rect = new BrowserRect();
  assert.deepEqual(Object.keys(rect), []);
  assert.deepEqual(restingDockIconTarget(rect, { x: 0, y: 1080 }, 1, { x: 0, y: 82 }),
    { left: 1800, top: 2080, right: 1850, bottom: 2130 });
});

test("hide translation tolerates numerical noise but never accepts a scaled or rotated root", () => {
  const matrix = { is2D: true, a: 1, b: 0, c: 0, d: 1, e: 0, f: 82.5 };
  assert.equal(isDockTranslation(matrix), true);
  assert.equal(isDockTranslation({ ...matrix, a: 1.0000001, b: -1e-8 }), true);
  for (const invalid of [
    { ...matrix, a: 0.99 }, { ...matrix, d: 1.01 },
    { ...matrix, b: 0.01 }, { ...matrix, c: 0.01 },
    { ...matrix, e: NaN }, { ...matrix, is2D: false },
  ]) assert.equal(isDockTranslation(invalid), false);
});

test("autohidden bottom Dock retains its resting icon target through the slide", () => {
  const origin = { x: 0, y: 1080 };
  const expected = { left: 1800, top: 2080, right: 1850, bottom: 2130 };
  for (const y of [0, 32.5, 82]) {
    assert.deepEqual(restingDockIconTarget(
      { left: 1800, top: 1000 + y, width: 50, height: 50 }, origin, 1, { x: 0, y },
    ), expected);
  }
});

test("left-side hidden Dock supports negative monitor origin and fractional DPI", () => {
  assert.deepEqual(restingDockIconTarget(
    { left: -70, top: 500, width: 48, height: 48 }, { x: -2560, y: 0 }, 1.5, { x: -80, y: 0 },
  ), { left: -2545, top: 750, right: -2473, bottom: 822 });
});

test("unmeasurable or excessive resting icon bounds never become a native target", () => {
  const icon = { left: 10, top: 20, width: 48, height: 48 };
  assert.equal(restingDockIconTarget(icon, { x: 0, y: 0 }, 1, { x: NaN, y: 0 }), null);
  assert.equal(restingDockIconTarget({ ...icon, width: 513 }, { x: 0, y: 0 }, 1, { x: 0, y: 0 }), null);
});
