import { strict as assert } from "node:assert";
import { test } from "node:test";
import { containsPoint, readHoverAnchor } from "./hoverBounds.ts";

test("Dock hover follows the physical icon across negative-origin and scaled displays", () => {
  const area = { left: -1400, top: 140, right: -640, bottom: 218 };
  assert.deepEqual(readHoverAnchor(area), area);
  assert.equal(containsPoint(area, -1400, 140), true);
  assert.equal(containsPoint(area, -641, 217), true);
  assert.equal(containsPoint(area, -640, 150), false);
  assert.equal(containsPoint(area, -1000, 218), false);
});

test("click-open and malformed hover triggers have no hover lifetime", () => {
  for (const value of [null, {}, { left: 2, right: 1, top: 0, bottom: 1 }]) {
    assert.equal(readHoverAnchor(value), null);
  }
});
