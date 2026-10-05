import { test } from "node:test";
import assert from "node:assert/strict";
import { appIconGeometry, appIconIsTile } from "./appIconGeometry.ts";

test("transparent and full-bleed icons remain safe", () => {
  assert.deepEqual(appIconGeometry(new Uint8Array(64 * 64 * 4), 64, 64), { scale: 1, x: 0, y: 0 });
  const full = new Uint8Array(64 * 64 * 4).fill(255);
  const result = appIconGeometry(full, 64, 64);
  assert.equal(result.x, 0);
  assert.equal(result.y, 0);
  assert.ok(result.scale <= 1);
});

test("square tiles fill the shared rounded mask at every display size", () => {
  const full = new Uint8Array(64 * 64 * 4).fill(255);
  assert.equal(appIconIsTile(full, 64, 64), true);
  assert.deepEqual(appIconGeometry(full, 64, 64, true), { scale: 1, x: 0, y: 0 });
});

test("circles and freeform marks get a plate, but existing rounded-square tiles do not", () => {
  const circle = new Uint8Array(64 * 64 * 4);
  const rounded = new Uint8Array(64 * 64 * 4);
  const narrow = new Uint8Array(64 * 64 * 4);
  for (let y = 0; y < 64; y++) {
    for (let x = 0; x < 64; x++) {
      const offset = (y * 64 + x) * 4 + 3;
      if (Math.hypot(x - 31.5, y - 31.5) < 30) circle[offset] = 255;
      if (Math.hypot(Math.max(Math.abs(x - 31.5) - 17, 0), Math.max(Math.abs(y - 31.5) - 17, 0)) < 13) {
        rounded[offset] = 255;
      }
      if (x >= 20 && x < 44) narrow[offset] = 255;
    }
  }
  assert.equal(appIconIsTile(circle, 64, 64), false);
  assert.equal(appIconIsTile(rounded, 64, 64), true);
  assert.equal(appIconIsTile(narrow, 64, 64), false);
  assert.equal(appIconIsTile(new Uint8Array(64 * 64 * 4), 64, 64), false);
  // A faint square shadow must not make a circular app icon full-bleed.
  for (let i = 3; i < circle.length; i += 4) if (!circle[i]) circle[i] = 32;
  assert.equal(appIconIsTile(circle, 64, 64), false);
});
test("normalizes padding without stretching aspect ratio or clipping source bounds", () => {
  const data = new Uint8Array(64 * 64 * 4);
  for (let y = 10; y < 42; y++) for (let x = 12; x < 44; x++) data[(y * 64 + x) * 4 + 3] = 255;
  const result = appIconGeometry(data, 64, 64);
  assert.ok(result.scale > 1 && result.scale <= 2);
  assert.ok(result.x > 0 && result.y > 0);
  assert.ok(32 / 64 * result.scale < 1);
});
