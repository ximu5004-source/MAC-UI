import { test } from "node:test";
import assert from "node:assert/strict";
import { clipRegion } from "./interactiveRegions.ts";

test("desktop regions are clipped to the viewport, not a fullscreen union", () => {
  assert.deepEqual(clipRegion([-5, 12, 40, 70], [0, 0, 100, 100]), [0, 12, 40, 70]);
  assert.equal(clipRegion([120, 12, 160, 70], [0, 0, 100, 100]), null);
  assert.equal(clipRegion([0, 0, NaN, 70], [0, 0, 100, 100]), null);
});
