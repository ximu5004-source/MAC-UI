import assert from "node:assert/strict";
import { test } from "node:test";
import type { SysTrayIcon } from "@seelen-ui/lib/types";
import { allTrayItems, trayIconKey, trayIconLabel, trayPanelSize } from "./trayPresentation.ts";

function icon(uid: number, overrides: Partial<SysTrayIcon> = {}): SysTrayIcon {
  return {
    stable_id: { HandleUid: [100, uid] },
    uid,
    window_handle: 100,
    guid: null,
    executable: null,
    tooltip: `App ${uid}`,
    icon_path: null,
    icon_handle: null,
    icon_image_hash: null,
    notification: null,
    notification_revision: 0,
    callback_message: 1024,
    version: 4,
    is_visible: true,
    ...overrides,
  };
}

test("full tray includes non-messaging, hidden, and native system icons", () => {
  const source = [
    icon(1),
    icon(2, { is_visible: false }),
    icon(3, { guid: "7820ae73-23e3-4229-82c1-e41cb67d5b9c" }),
    icon(4, { executable: "C:\\Apps\\Weixin.exe", is_visible: false }),
  ];
  const result = allTrayItems(source);
  assert.equal(result.length, 4);
  assert.equal(result[0]?.uid, 4);
  assert.deepEqual(new Set(result.map(trayIconKey)), new Set(source.map(trayIconKey)));
  assert.deepEqual(source.map((item) => item.uid), [1, 2, 3, 4]);
});

test("only duplicate stable IDs are merged, not different icons from the same app", () => {
  const result = allTrayItems([
    icon(1, { tooltip: "App" }),
    icon(2, { tooltip: "App" }),
    icon(1, { tooltip: "App updated", is_visible: false }),
  ]);
  assert.equal(result.length, 2);
  assert.equal(result.find((item) => item.uid === 1)?.tooltip, "App updated");
  assert.equal(result.find((item) => item.uid === 1)?.is_visible, false);
});

test("unnamed icons keep a useful executable fallback and are not dropped", () => {
  assert.equal(trayIconLabel(icon(1, { tooltip: " ", executable: "C:\\Apps\\PixPin.exe" })), "PixPin");
  assert.equal(trayIconLabel(icon(2, { tooltip: "", executable: null })), "");
  assert.equal(allTrayItems([icon(2, { tooltip: "", executable: null })]).length, 1);
});

test("tray dimensions use monitor work area with DPI scaling, not popup viewport", () => {
  assert.deepEqual(trayPanelSize(3840, 2100), { width: 360, maxHeight: 600 });
  assert.deepEqual(trayPanelSize(1920, 1080, 2), { width: 360, maxHeight: 476 });
  assert.deepEqual(trayPanelSize(320, 480), { width: 288, maxHeight: 416 });
  assert.deepEqual(trayPanelSize(NaN, NaN, 0), { width: 360, maxHeight: 600 });
});
