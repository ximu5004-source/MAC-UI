import assert from "node:assert/strict";
import { test } from "node:test";
import { launchpadBounds } from "./launchpadGeometry.ts";

test("fullscreen uses every pixel of the target monitor regardless of DPI and origin", () => {
  for (const monitor of [
    { rect: { left: 0, top: 0, right: 1920, bottom: 1080 }, scaleFactor: 1 },
    { rect: { left: 1920, top: -400, right: 5760, bottom: 1760 }, scaleFactor: 1.5 },
    { rect: { left: -3840, top: 0, right: 0, bottom: 2160 }, scaleFactor: 2 },
    { rect: { left: 0, top: -1920, right: 1080, bottom: 0 }, scaleFactor: 1.25 },
  ]) {
    assert.deepEqual(launchpadBounds(monitor, true), monitor.rect);
  }
});

test("windowed mode remains centered on the chosen mixed-DPI monitor", () => {
  const monitor = { rect: { left: -3840, top: 0, right: 0, bottom: 2160 }, scaleFactor: 2 };
  assert.deepEqual(launchpadBounds(monitor, false), { left: -3020, top: 230, right: -820, bottom: 1930 });
  assert.deepEqual(launchpadBounds({ rect: { left: 0, top: 0, right: 800, bottom: 600 }, scaleFactor: 1.5 }, false),
    { left: 40, top: 36, right: 760, bottom: 564 });
});

test("invalid monitor geometry is rejected instead of positioning outside the desktop", () => {
  for (const rect of [
    { left: 0, top: 0, right: 0, bottom: 1080 },
    { left: 0, top: 0, right: 1920, bottom: -1 },
    { left: NaN, top: 0, right: 1920, bottom: 1080 },
  ]) assert.equal(launchpadBounds({ rect, scaleFactor: 1 }, true), null);
});
