import assert from "node:assert/strict";
import test from "node:test";
import type { Dialog, DialogContent } from "@seelen-ui/lib/types";
import { shortcutCancelEvent } from "./shortcutClose";

function fixture(id: string, onClick: string): Dialog {
  const footer: DialogContent[] = [{ type: "button", onClick, inner: [] }];
  return { identifier: id, title: [], content: [], footer, width: 360, height: 180 };
}

test("header close cancels only its own shortcut registration", () => {
  const event = "shortcut_register_cancelled:dialog-b";
  assert.equal(shortcutCancelEvent(fixture("dialog-b", event)), event);
});

test("old or unrelated cancel events do not change generic dialog closing", () => {
  assert.equal(shortcutCancelEvent(fixture("dialog-b", "shortcut_register_cancelled:dialog-a")), null);
  assert.equal(shortcutCancelEvent(fixture("dialog-b", "shortcut_register_cancelled")), null);
  assert.equal(shortcutCancelEvent(fixture("dialog-b", "exit")), null);
  assert.equal(shortcutCancelEvent(fixture("dialog-b", "open_settings_shortcuts")), null);
  assert.equal(shortcutCancelEvent(null), null);
});
