import assert from "node:assert/strict";
import { test } from "node:test";
import { TrayAttention, TRAY_ACTIVITY_WINDOW_MS } from "./trayAttention.ts";

const item = (hash: string, notificationKeys: string[] = [], id = "wechat") => ({ id, hash, notificationKeys });

test("initial images, existing notifications and unchanged HICON snapshots never start attention", () => {
  const state = new TrayAttention();
  state.observe([item("a", ["old-toast"])], 0);
  for (let n = 1; n < 10; n++) state.observe([item("a", ["old-toast"])], n * 100);
  assert.equal(state.snapshot(1000).flashing.size, 0);
});

test("successive native pixel changes produce activity and expire when Windows stops changing", () => {
  const state = new TrayAttention();
  state.observe([item("a")], 0);
  state.observe([item("b")], 500);
  state.observe([item("a")], 1000);
  assert.equal(state.snapshot(1000).active.size, 0);
  state.observe([item("b")], 1500);
  assert.equal(state.snapshot(1500).active.has("wechat"), true);
  assert.equal(state.snapshot(1500).flashing.has("wechat"), true);
  assert.equal(state.snapshot(1500 + TRAY_ACTIVITY_WINDOW_MS).flashing.size, 0);
});

test("explicit blank native frames participate in flashing without creating notification content", () => {
  const state = new TrayAttention();
  ["app-pixels", "native:null-icon", "app-pixels", "native:null-icon"].forEach((hash, n) =>
    state.observe([item(hash)], n * 500)
  );
  assert.equal(state.snapshot(1500).active.has("wechat"), true);
  assert.equal(state.snapshot(1500).flashing.has("wechat"), true);
  assert.equal(state.snapshot(1500 + TRAY_ACTIVITY_WINDOW_MS).flashing.size, 0);
});

test("click stops the current native burst and a later separate burst can alert again", () => {
  const state = new TrayAttention();
  ["a", "b", "a", "b"].forEach((hash, n) => state.observe([item(hash)], n * 500));
  state.acknowledge("wechat");
  ["a", "b", "a", "b"].forEach((hash, n) => state.observe([item(hash)], 2000 + n * 500));
  assert.equal(state.snapshot(4000).flashing.size, 0);
  ["a", "b", "a"].forEach((hash, n) => state.observe([item(hash)], 12000 + n * 500));
  assert.equal(state.snapshot(13000).flashing.has("wechat"), true);
});

test("new notification starts attention, removals and initial permission fetch do not", () => {
  const state = new TrayAttention();
  state.observe([item("a")], 0);
  state.observe([item("a", ["old-toast"])], 100, true);
  assert.equal(state.snapshot(100).flashing.size, 0);
  state.observe([item("a", ["old-toast", "new-toast"])], 200);
  assert.equal(state.snapshot(200).flashing.has("wechat"), true);
  state.acknowledge("wechat");
  state.observe([item("a", ["new-toast"])], 300);
  assert.equal(state.snapshot(300).flashing.size, 0);
  state.observe([item("a", ["new-toast", "later-toast"])], 400);
  assert.equal(state.snapshot(400).flashing.has("wechat"), true);
  assert.equal(state.snapshot(15400).flashing.size, 0);
  state.observe([item("a")], 15500);
  assert.equal(state.snapshot(15500).nextDeadline, null);
});

test("removed owners are forgotten and relaunch is baselined", () => {
  const state = new TrayAttention();
  state.observe([item("a")], 0);
  state.observe([item("a", ["new"])], 100);
  state.observe([], 200);
  assert.equal(state.snapshot(200).flashing.size, 0);
  state.observe([item("b", ["new"])], 300);
  assert.equal(state.snapshot(300).flashing.size, 0);
});
