// Syntax/data acceptance only. This never creates a browser or invokes native APIs.
import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import test from "node:test";

let fixture;
try {
  fixture = JSON.parse(await readFile(new URL("../../target/qa/genie-geometry.json", import.meta.url), "utf8"));
} catch (error) {
  // Generated artifacts are intentionally not committed. Fresh frontend-only
  // checks still validate module syntax; explicit geometry QA exports it first.
  if (error.code !== "ENOENT") throw error;
}
const generatedDataOnly = { skip: !fixture && "Export the owned Rust geometry fixture before explicit data QA" };
const html = await readFile(new URL("./genie-preview.html", import.meta.url), "utf8");

test("preview module parses without executing page or native code", () => {
  const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];
  assert.ok(script);
  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
  assert.doesNotThrow(() => new AsyncFunction(script));
  assert.match(html, /prefers-reduced-motion/);
  assert.match(html, /id="stop">停止/);
  assert.match(html, /此预览通过不代表 Windows 原生动画去重/);
});

test("generated four-direction geometry has bounded, finite complete crops", generatedDataOnly, () => {
  assert.equal(fixture.generator, "src/background/widgets/weg/genie/geometry.rs");
  assert.match(fixture.scope, /no native capture/);
  assert.equal(fixture.neckPhaseEnd, 0.42);
  assert.equal(fixture.durationMs, 420);
  assert.deepEqual(fixture.cases.map(entry => entry.name), ["bottom", "top", "left", "right"]);
  const [sx, sy, sw, sh] = fixture.source;
  for (const entry of fixture.cases) {
    assert.equal(entry.frames.length, 101);
    for (let step = 0; step <= 100; step++) {
      const frame = entry.frames[step];
      assert.equal(frame.progress, step / 100);
      assert.equal(frame.strips.length, 256);
      for (const strip of frame.strips) {
        assert.equal(strip.length, 8);
        assert.ok(strip.every(Number.isFinite));
        const [x, y, w, h, dx, dy, dw, dh] = strip;
        assert.ok(x >= 0 && y >= 0 && w > 0 && h > 0 && x + w <= sw && y + h <= sh);
        assert.ok(dw > 0 && dh > 0 && dx >= 0 && dy >= 0 && dx + dw <= fixture.canvas[0] && dy + dh <= fixture.canvas[1]);
      }
    }
    const first = entry.frames[0].strips;
    assert.equal(first[0][4], sx);
    assert.equal(first[0][5], sy);
  }
});

test("fixture retains two-stage boundary anchoring instead of uniform scale", generatedDataOnly, () => {
  for (const entry of fixture.cases) {
    const horizontal = ["left", "right"].includes(entry.name);
    const reverse = ["left", "top"].includes(entry.name);
    const axis = horizontal ? 4 : 5;
    const span = horizontal ? 6 : 7;
    const crossSpan = horizontal ? 7 : 6;
    const sourceAxis = fixture.source[horizontal ? 0 : 1];
    const sourceLength = fixture.source[horizontal ? 2 : 3];
    const targetAxis = entry.target[horizontal ? 0 : 1];
    const targetLength = entry.target[horizontal ? 2 : 3];
    const neck = entry.frames[42].strips;
    const far = reverse ? neck.at(-1) : neck[0];
    const near = reverse ? neck[0] : neck.at(-1);
    const farBoundary = reverse ? far[axis] + far[span] : far[axis];
    assert.ok(Math.abs(farBoundary - (sourceAxis + (reverse ? sourceLength : 0))) < 1e-9);
    assert.ok(far[crossSpan] > near[crossSpan] * 4, "funnel must be non-affine");
    for (let step = 42; step <= 100; step++) {
      const strips = entry.frames[step].strips;
      const tip = reverse ? strips[0] : strips.at(-1);
      const tipBoundary = reverse ? tip[axis] : tip[axis] + tip[span];
      assert.ok(Math.abs(tipBoundary - (targetAxis + (reverse ? 0 : targetLength))) < 1e-9);
    }
  }
});
