import assert from "node:assert/strict";
import { test } from "node:test";
import { pageDirection, WheelPager } from "./pageMotion.ts";

test("page direction tracks fast forward/reverse/jumps without an animation queue", () => {
  const at = (page: number) => ({ page, resetKey: "grid:35" });
  assert.equal(pageDirection(undefined, at(0)), 0);
  assert.equal(pageDirection(at(0), at(1)), 1);
  assert.equal(pageDirection(at(1), at(5)), 1);
  assert.equal(pageDirection(at(5), at(1)), -1);
  assert.equal(pageDirection(at(1), at(1)), 0);
});

test("search, resize, reopening and folder changes never animate from stale content", () => {
  for (const resetKey of ["search:query", "grid:12", "open:2", "folder:other"]) {
    assert.equal(pageDirection({ page: 3, resetKey: "old" }, { page: 0, resetKey }), 0);
  }
});

const wheel = (deltaX = 0, deltaY = 0, deltaMode = 0, ctrlKey = false) => ({ deltaX, deltaY, deltaMode, ctrlKey });

test("precision touchpad deltas accumulate into exactly one page including long momentum", () => {
  const pager = new WheelPager();
  const steps = Array.from({ length: 80 }, (_, index) => pager.consume(wheel(5), index * 16, 1000));
  assert.equal(steps.filter(Boolean).length, 1);
  assert.equal(steps[7], 1);
  assert.equal(pager.consume(wheel(40), 1700, 1000), 1);
});

test("wheel direction reversal responds immediately and modifiers preserve zoom", () => {
  const pager = new WheelPager();
  assert.equal(pager.consume(wheel(0, 120), 0, 1000), 1);
  assert.equal(pager.consume(wheel(0, -120), 20, 1000), -1);
  assert.equal(pager.consume(wheel(0, 120, 0, true), 40, 1000), 0);
  assert.equal(pager.consume(wheel(0, 120), 60, 1000), 1);
});

test("line/page wheels, diagonal gestures, reset and invalid input are bounded", () => {
  const pager = new WheelPager();
  assert.equal(pager.consume(wheel(0, 3, 1), 0, 1000), 1);
  assert.equal(pager.consume(wheel(-1, 0, 2), 10, 1000), -1);
  pager.reset();
  assert.equal(pager.consume(wheel(45, -20), 20, 1000), 1);
  assert.equal(pager.consume(wheel(NaN, NaN), 30, 1000), 0);
  assert.equal(pager.consume(wheel(), 40, 1000), 0);
});
