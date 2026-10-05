import { strict as assert } from "node:assert";
import { test } from "node:test";
import { frostedForeground } from "./frostedContrast";

test("glass text adapts to bright/dark wallpaper without tinting the material", () => {
  assert.equal(frostedForeground(0), "white");
  assert.equal(frostedForeground(1), "black");
  assert.equal(frostedForeground(.4), "black");
  assert.equal(frostedForeground(.05), "white");
});
test("neutral coating is included and transient sampling errors keep previous ink", () => {
  assert.equal(frostedForeground(.17), "black");
  assert.equal(frostedForeground(NaN, "black"), "black");
  assert.equal(frostedForeground(Infinity, "white"), "white");
});
