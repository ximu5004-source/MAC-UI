import assert from "node:assert/strict";
import test from "node:test";
import type { AppNotification, CommunicationNotification } from "@macui/lib/types";
import { mergeNotificationCenter, NotificationFlyoutTracker } from "./notificationCenter.ts";

const NOW = 1_800_000_000_000;
function native(id: number, date = NOW, name = "ChatGPT", texts = ["Ready"]): AppNotification {
  return {
    id,
    date,
    appName: name,
    appUmid: name,
    appDescription: "",
    content: { visual: { binding: { $value: texts.map((text) => ({ text: { $value: text } })) } } },
  } as AppNotification;
}
function bridge(id: string, date = NOW, source = "Balloon", app = "WeChat"): CommunicationNotification {
  return {
    id,
    date,
    app,
    source,
    trayId: { uid: 1, window: 1 },
    iconPath: null,
    title: "Alice",
    body: "Meeting at 2 pm",
  } as unknown as CommunicationNotification;
}

test("notification IDs are source-prefixed, sorted, and do not mutate original arrays", () => {
  const a = [native(1, NOW - 200), native(2, NOW + 200)];
  const b = [bridge("1", NOW)];
  const merged = mergeNotificationCenter(a, b);
  assert.deepEqual(merged.map((item) => item.key), ["windows:2", "communication:1", "windows:1"]);
  assert.equal(a[0]?.id, 1);
  assert.equal(b[0]?.id, "1");
  assert.equal(merged[0]?.notification, a[1]);
});

test("native full toast wins over a matching real balloon while unrelated messages remain", () => {
  const toast = native(1, NOW, "微信", ["Alice", "Meeting at 2 pm"]);
  const merged = mergeNotificationCenter([toast], [bridge("same"), { ...bridge("other"), body: "Different message" }]);
  assert.deepEqual(merged.map((item) => item.key), ["communication:other", "windows:1"]);
  assert.equal(merged.find((item) => item.source === "windows")?.notification, toast);
});

test("nearby unread hints do not duplicate native messages, old native history does not hide new hints", () => {
  const hint = bridge("unread", NOW, "UnreadHint");
  assert.equal(mergeNotificationCenter([native(1, NOW, "WeChat")], [hint]).length, 1);
  assert.equal(mergeNotificationCenter([native(1, NOW - 16_000, "WeChat")], [hint]).length, 2);
  assert.equal(mergeNotificationCenter([native(1, NOW, "WeCom")], [hint]).length, 2);
});

test("identical body from two distinct messaging apps is not merged", () => {
  assert.equal(
    mergeNotificationCenter([native(1, NOW, "WeCom", ["Alice", "Meeting at 2 pm"])], [bridge("wx")]).length,
    2,
  );
});

test("all four app popup sources deduplicate against real native Toast content", () => {
  for (const [app, label] of [["WeChat", "微信"], ["WeCom", "企业微信"], ["DingTalk", "钉钉"], ["Feishu", "飞书"]]) {
    const popup = {
      ...bridge(`popup-${app}`, NOW, "AccessiblePopup", app),
      trayId: null,
      title: label!,
      body: "Alice\nMeeting at 2 pm",
    };
    const toast = native(1, NOW + 10, label, ["Alice", "Meeting at 2 pm"]);
    assert.deepEqual(mergeNotificationCenter([toast], [popup]).map((entry) => entry.key), ["windows:1"]);
    const tracker = new NotificationFlyoutTracker();
    tracker.consume([], NOW, true);
    assert.equal(tracker.consume(mergeNotificationCenter([], [popup]), NOW, true)?.key, `communication:popup-${app}`);
    assert.equal(tracker.consume(mergeNotificationCenter([toast], [popup]), NOW + 10, true), null);
  }
});

test("startup list is a baseline; dismissal cannot replay an older item", () => {
  const tracker = new NotificationFlyoutTracker();
  const old = mergeNotificationCenter([native(1, NOW - 100)], []);
  assert.equal(tracker.consume(old, NOW, true), null);
  const next = mergeNotificationCenter([native(1, NOW - 100), native(2, NOW + 1)], []);
  assert.equal(tracker.consume(next, NOW + 1, true)?.key, "windows:2");
  assert.equal(tracker.consume(old, NOW + 2, true), null);
  assert.equal(tracker.consume(next, NOW + 3, true), null);
});

test("DND or disabled popups consume IDs silently and do not replay when re-enabled", () => {
  const tracker = new NotificationFlyoutTracker();
  tracker.consume([], NOW, false);
  const next = mergeNotificationCenter([], [bridge("during-dnd", NOW + 1)]);
  assert.equal(tracker.consume(next, NOW + 1, false), null);
  assert.equal(tracker.consume(next, NOW + 2, true), null);
  const newItem = mergeNotificationCenter([], [bridge("later", NOW + 3), bridge("during-dnd", NOW + 1)]);
  assert.equal(tracker.consume(newItem, NOW + 3, true)?.key, "communication:later");
});

test("slow or denied Windows initialization does not block independent messaging popups", () => {
  const tracker = new NotificationFlyoutTracker();
  const ready = { windows: false, communication: true };
  tracker.consume([], NOW, true, ready);
  assert.equal(
    tracker.consume(mergeNotificationCenter([], [bridge("new", NOW + 1)]), NOW + 1, true, ready)?.key,
    "communication:new",
  );
  const lateHistory = mergeNotificationCenter([native(7, NOW + 2)], [bridge("new", NOW + 1)]);
  assert.equal(tracker.consume(lateHistory, NOW + 2, true, { windows: true, communication: true }), null);
});

test("a new tray message waiting for quiet-mode lookup is shown once only if permitted", () => {
  for (const allowed of [true, false]) {
    const tracker = new NotificationFlyoutTracker();
    const history = mergeNotificationCenter([], [bridge("old", NOW)]);
    tracker.consume(history, NOW, null);
    const items = mergeNotificationCenter([], [bridge("new", NOW + 1, "TrayActivity"), bridge("old", NOW)]);
    const keys = items.map((item) => item.key);
    assert.equal(tracker.consume(items, NOW + 1, null, undefined, keys), null);
    assert.equal(tracker.consume(items, NOW + 2, allowed)?.key, allowed ? "communication:new" : undefined);
    assert.equal(tracker.consume(items, NOW + 3, true), null);
  }
});

test("stale or future notification dates do not create automatic popups", () => {
  const tracker = new NotificationFlyoutTracker();
  tracker.consume([], NOW, true);
  assert.equal(
    tracker.consume(mergeNotificationCenter([native(1, NOW - 20_000), native(2, NOW + 50)], []), NOW, true),
    null,
  );
});

test("late native duplicate does not replay a balloon, but a repeated real message can still appear", () => {
  const tracker = new NotificationFlyoutTracker();
  tracker.consume([], NOW, true);
  const balloon = bridge("balloon", NOW + 1);
  assert.equal(tracker.consume(mergeNotificationCenter([], [balloon]), NOW + 1, true)?.key, "communication:balloon");
  const toast = native(2, NOW + 2, "WeChat", ["Alice", "Meeting at 2 pm"]);
  assert.equal(tracker.consume(mergeNotificationCenter([toast], [balloon]), NOW + 2, true), null);
  assert.equal(
    tracker.consume(mergeNotificationCenter([], [bridge("another", NOW + 3), balloon]), NOW + 3, true)?.key,
    "communication:another",
  );
});

test("dismissing a native copy cannot reveal its suppressed bridge duplicate as a new popup", () => {
  const tracker = new NotificationFlyoutTracker();
  tracker.consume([], NOW, true);
  const toast = native(2, NOW + 1, "WeChat", ["Alice", "Meeting at 2 pm"]);
  const balloon = bridge("duplicate", NOW + 1);
  const ready = { windows: true, communication: true };
  const keys = ["windows:2", "communication:duplicate"];
  assert.equal(
    tracker.consume(mergeNotificationCenter([toast], [balloon]), NOW + 1, true, ready, keys)?.key,
    "windows:2",
  );
  assert.equal(
    tracker.consume(mergeNotificationCenter([], [balloon]), NOW + 2, true, ready, ["communication:duplicate"]),
    null,
  );
});

for (const app of ["WeChat", "WeCom"]) {
  test(app + " tray-only signal opens one flyout and yields to a real Windows notification", () => {
    const tracker = new NotificationFlyoutTracker();
    tracker.consume([], NOW, true);
    const hint = { ...bridge("burst", NOW, "TrayActivity", app), title: "", body: "" };
    const cards = mergeNotificationCenter([], [hint]);
    assert.equal(tracker.consume(cards, NOW, true)?.key, "communication:burst");
    assert.equal(tracker.consume(cards, NOW + 500, true), null);
    assert.deepEqual(mergeNotificationCenter([native(2, NOW + 700, app)], [hint]).map(n => n.key), ["windows:2"]);
  });
}
