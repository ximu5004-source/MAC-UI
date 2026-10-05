import { strict as assert } from "node:assert";
import { test } from "node:test";
import { DOCK_SIZE_DEFAULT, normalizeDockSize } from "../../utils/dockSize.ts";

test("Dock size keeps existing valid settings and clamps slider endpoints", () => {
  for (const size of [16, 24, 40, 64, 96, 128]) assert.equal(normalizeDockSize(size), size);
  assert.equal(normalizeDockSize(0), 16);
  assert.equal(normalizeDockSize(-100), 16);
  assert.equal(normalizeDockSize(1000), 128);
  assert.equal(normalizeDockSize(63.5), 64);
});

test("Invalid Dock size cannot collapse icon cells or the native hitbox", () => {
  for (const size of [undefined, null, "64", NaN, Infinity, -Infinity, {}, []]) {
    assert.equal(normalizeDockSize(size), DOCK_SIZE_DEFAULT);
  }
});
