import assert from "node:assert/strict";
import { test } from "node:test";
import { alignmentGridVisible, clampStackPosition, clampStackSize, defaultStackPosition, isStackPoint, isStackSize, moveStackPosition, resizeStackRect } from "./stackLayout.ts";

test("visible grid is independent from snapping and keeps automatic drag guides", () => {
  assert.equal(alignmentGridVisible(false, false, true), false);
  assert.equal(alignmentGridVisible(true, false, false), true);
  assert.equal(alignmentGridVisible(false, true, true), true);
  assert.equal(alignmentGridVisible(false, true, false), false);
});

test("grid snapping aligns moves, free mode preserves exact coordinates", () => {
  const bounds = { width: 1000, height: 800 }, size = { width: 260, height: 320 };
  assert.deepEqual(moveStackPosition({ x: 133, y: 147 }, bounds, size, true), { x: 140, y: 140 });
  assert.deepEqual(moveStackPosition({ x: 133, y: 147 }, bounds, size, false), { x: 133, y: 147 });
  assert.deepEqual(moveStackPosition({ x: 1000, y: 1000 }, bounds, size, true), { x: 740, y: 480 });
});

test("resize snaps moving edges and preserves the opposite corner", () => {
  const bounds = { width: 1000, height: 800 };
  const rect = { x: 100, y: 100, width: 260, height: 320 };
  assert.deepEqual(resizeStackRect(rect, { x: 133, y: 87 }, "se", bounds, true), { x: 100, y: 100, width: 400, height: 400 });
  assert.deepEqual(resizeStackRect(rect, { x: -39, y: -27 }, "nw", bounds, true), { x: 60, y: 80, width: 300, height: 340 });
  assert.deepEqual(resizeStackRect(rect, { x: 13, y: 7 }, "se", bounds, false), { x: 100, y: 100, width: 273, height: 327 });
});

test("resize respects minimum readable size and available desktop", () => {
  const bounds = { width: 1000, height: 800 };
  const rect = { x: 100, y: 100, width: 260, height: 320 };
  assert.deepEqual(resizeStackRect(rect, { x: -999, y: -999 }, "se", bounds, true), { x: 100, y: 100, width: 160, height: 200 });
  assert.deepEqual(resizeStackRect(rect, { x: 9999, y: 9999 }, "se", bounds, true), { x: 100, y: 100, width: 900, height: 700 });
  assert.deepEqual(resizeStackRect(rect, { x: 9999, y: 9999 }, "nw", bounds, true), { x: 200, y: 220, width: 160, height: 200 });
  assert.deepEqual(clampStackSize({ width: 9999, height: 9999 }, { width: 300, height: 400 }), { width: 300, height: 400 });
  assert.deepEqual(clampStackSize({ width: 1, height: 1 }, { width: 100, height: 100 }), { width: 100, height: 100 });
  assert.equal(isStackSize({ width: 300, height: 400 }), true);
  assert.equal(isStackSize({ width: NaN, height: 400 }), false);
  assert.equal(isStackSize({ width: -10, height: 400 }), false);
});

test("dragging keeps the entire stack inside the desktop", () => {
  assert.deepEqual(clampStackPosition({ x: 5000, y: -30 }, { width: 1920, height: 900 }, { width: 260, height: 400 }), {
    x: 1660,
    y: 0,
  });
  assert.deepEqual(clampStackPosition({ x: 400, y: 700 }, { width: 200, height: 300 }, { width: 260, height: 400 }), {
    x: 0,
    y: 0,
  });
});

test("default stacks start at the right and wrap on a narrow desktop", () => {
  assert.deepEqual(defaultStackPosition(0, { width: 1000, height: 800 }), { x: 740, y: 0 });
  assert.deepEqual(defaultStackPosition(3, { width: 1000, height: 800 }), { x: 740, y: 400 });
});

test("corrupt saved coordinates are ignored", () => {
  assert.equal(isStackPoint({ x: 10, y: 20 }), true);
  assert.equal(isStackPoint({ x: "10", y: 20 }), false);
  assert.equal(isStackPoint({ x: Infinity, y: 20 }), false);
  assert.equal(isStackPoint(null), false);
});
