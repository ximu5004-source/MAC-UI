import assert from "node:assert/strict";
import { test } from "node:test";
import { LatestPlacement, LatestToggle } from "./launchpadOperations.ts";

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>(done => { resolve = done; });
  return { promise, resolve };
}

test("queued geometry coalesces to the latest and active placement never overlaps", async () => {
  const entered = deferred(), finish = deferred();
  const applied: number[] = [];
  const placement = new LatestPlacement<number>(async value => {
    applied.push(value);
    if (value === 1) { entered.resolve(); await finish.promise; }
  });
  const first = placement.request(1);
  await entered.promise;
  const second = placement.request(2);
  const third = placement.request(3);
  assert.deepEqual(applied, [1]);
  finish.resolve();
  await Promise.all([first, second, third, placement.settled()]);
  assert.deepEqual(applied, [1, 3]);
});

test("rapid double/triple triggers preserve toggle intent without stale show calls", async () => {
  const applied: [boolean, number][] = [];
  const visibility = new LatestToggle<number>(async () => false, async (visible, input) => { applied.push([visible, input]); });
  await Promise.all([visibility.toggle(1), visibility.toggle(2)]);
  assert.deepEqual(applied, [[false, 2]]);
  applied.length = 0;
  await Promise.all([visibility.toggle(3), visibility.toggle(4), visibility.toggle(5)]);
  assert.deepEqual(applied, [[true, 5]]);
});

test("new hide intent invalidates a show still waiting for placement", async () => {
  const entered = deferred(), finish = deferred();
  const applied: boolean[] = [];
  const visibility = new LatestToggle<number>(async () => false, async (visible, _input, isCurrent) => {
    if (visible) { entered.resolve(); await finish.promise; }
    if (isCurrent()) applied.push(visible);
  });
  const showing = visibility.toggle(1);
  await entered.promise;
  const hiding = visibility.toggle(2);
  finish.resolve();
  await Promise.all([showing, hiding]);
  assert.deepEqual(applied, [false]);
});

test("a delayed native hide cannot finish after a newer show", async () => {
  const entered = deferred(), finish = deferred();
  const applied: boolean[] = [];
  const visibility = new LatestToggle<void>(async () => true, async visible => {
    if (!visible) { entered.resolve(); await finish.promise; }
    applied.push(visible);
  });
  const hiding = visibility.toggle();
  await entered.promise;
  const showing = visibility.toggle();
  assert.deepEqual(applied, []);
  finish.resolve();
  await Promise.all([hiding, showing]);
  assert.deepEqual(applied, [false, true]);
});

test("external close is observed on next trigger and failed operations remain retryable", async () => {
  let nativeVisible = false;
  const applied: boolean[] = [];
  const visibility = new LatestToggle<void>(async () => nativeVisible, async visible => {
    applied.push(visible); nativeVisible = visible;
  });
  await visibility.toggle();
  nativeVisible = false;
  await visibility.toggle();
  assert.deepEqual(applied, [true, true]);
  let attempts = 0;
  const placement = new LatestPlacement<void>(async () => { if (++attempts === 1) throw new Error("unavailable"); });
  await assert.rejects(placement.request(), /unavailable/);
  await placement.request();
  assert.equal(attempts, 2);
});
