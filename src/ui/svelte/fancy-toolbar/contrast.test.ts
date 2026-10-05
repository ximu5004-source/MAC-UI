import { strict as assert } from "node:assert";
import { test } from "node:test";
import { toolbarForeground } from "./contrast.ts";

test("toolbar selects black on bright wallpaper and white on dark wallpaper", () => {
  assert.equal(toolbarForeground(0), "white");
  assert.equal(toolbarForeground(1), "black");
  assert.equal(toolbarForeground(0.18), "black");
});
test("small brightness changes do not make toolbar text flicker", () => {
  assert.equal(toolbarForeground(0.19, "white"), "white");
  assert.equal(toolbarForeground(0.17, "black"), "black");
  assert.equal(toolbarForeground(0.21, "white"), "black");
  assert.equal(toolbarForeground(0.15, "black"), "white");
  assert.equal(toolbarForeground(NaN, "black"), "black");
});
