import { test } from "node:test";
import assert from "node:assert/strict";
import { ICON_SIZES, validIconSize } from "./iconSize.ts";
test("old and corrupt layouts retain the default icon size", () => {
  for (const value of [undefined, null, -1, Infinity, NaN, "80", {}, 500]) assert.equal(validIconSize(value), 64);
  for (const size of ICON_SIZES) assert.equal(validIconSize(size), size);
});
