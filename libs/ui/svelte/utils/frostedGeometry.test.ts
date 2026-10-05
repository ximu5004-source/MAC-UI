import { strict as assert } from "node:assert";
import { test } from "node:test";
import { frostedRect, FrostedWriter, type FrostedSnapshot } from "./frostedGeometry";

const visible: FrostedSnapshot = { viewport: [800, 600], rects: [[10, 20, 210, 120, 22]] };
const hidden: FrostedSnapshot = { viewport: [800, 600], rects: [] };
test("frost follows the paint boundary without shadow or input gutters", () => {
  assert.deepEqual(frostedRect({ left: 10, top: 20, right: 210, bottom: 120 }, "22px", [800, 600]), visible.rects[0]);
});
test("offscreen motion retains original corners until fully hidden", () => {
  assert.deepEqual(frostedRect({ left: 10, top: 590, right: 210, bottom: 640 }, "50%", [800, 600]), [10, 590, 210, 640, 25]);
  assert.equal(frostedRect({ left: 10, top: 600, right: 210, bottom: 650 }, "22px", [800, 600]), null);
});
test("invalid surfaces are rejected and radius is clamped", () => {
  assert.equal(frostedRect({ left: NaN, top: 0, right: 200, bottom: 100 }, "22px", [800, 600]), null);
  assert.equal(frostedRect({ left: 1, top: 0, right: 1, bottom: 100 }, "22px", [800, 600]), null);
  assert.deepEqual(frostedRect({ left: 0, top: 0, right: 20, bottom: 10 }, "99px", [800, 600]), [0, 0, 20, 10, 5]);
});
test("a pending native show cannot overtake a later hide", async () => {
  const sent: FrostedSnapshot[] = [];
  let release!: () => void;
  const writer = new FrostedWriter(async (snapshot) => {
    sent.push(snapshot);
    if (sent.length === 1) await new Promise<void>((resolve) => { release = resolve; });
  });
  const pending = writer.write(visible);
  await Promise.resolve();
  void writer.write(hidden);
  release();
  await pending;
  assert.deepEqual(sent, [visible, hidden]);
  await writer.write(hidden);
  assert.equal(sent.length, 2);
});
test("deduplicated show immediately followed by hide cannot strand pending work", async () => {
  const sent: FrostedSnapshot[] = [];
  const writer = new FrostedWriter(async (snapshot) => { sent.push(snapshot); });
  await writer.write(visible);
  const show = writer.write(visible);
  const hide = writer.write(hidden);
  await Promise.all([show, hide]);
  assert.deepEqual(sent, [visible, hidden]);
});
test("failure remains retryable instead of marking native geometry as applied", async () => {
  let calls = 0;
  const writer = new FrostedWriter(async () => { if (++calls === 1) throw new Error("unavailable"); });
  await assert.rejects(writer.write(visible), /unavailable/);
  await writer.write(visible);
  assert.equal(calls, 2);
});
test("display recovery resubmits identical geometry", async () => {
  let calls = 0;
  const writer = new FrostedWriter(async () => { calls++; });
  await writer.write(visible);
  writer.invalidate();
  await writer.write(visible);
  assert.equal(calls, 2);
});
test("recovery during an in-flight update cannot cache stale geometry", async () => {
  let calls = 0, release!: () => void;
  const writer = new FrostedWriter(async () => {
    if (++calls === 1) await new Promise<void>((resolve) => { release = resolve; });
  });
  const pending = writer.write(visible);
  await Promise.resolve();
  writer.invalidate();
  void writer.write(visible);
  release();
  await pending;
  assert.equal(calls, 2);
});
