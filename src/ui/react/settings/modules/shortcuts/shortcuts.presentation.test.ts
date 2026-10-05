import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const view = readFileSync(new URL("./infrastructure.tsx", import.meta.url), "utf8");

test("each edit wires the same unique callback event to listen and invoke", () => {
  assert.match(view, /const callbackEvent = createShortcutCaptureEvent\(\);/);
  assert.match(view, /\.webview\.listen<null \| string\[\]>\(\s*callbackEvent,/);
  assert.match(view, /RequestToUserInputShortcut, \{ callbackEvent \}/);
  assert.doesNotMatch(view, /callbackEvent:\s*["']finished["']/);
});

test("Launchpad has an explicit opt-in switch and accessible shortcut controls", () => {
  assert.match(view, /widget\.id === "@seelen\/apps-menu" &&/);
  assert.match(view, /value=\{isWidgetEnabled\(widget\.id\)\}/);
  assert.match(view, /onChange=\{\(enabled\) => patchWidgetConfig\(widget\.id, \{ enabled \}\)\}/);
  assert.match(view, /aria-label=\{t\("widget\.enable"\)\}/);
  assert.match(view, /aria-label=\{t\("shortcuts\.edit"\)\}/);
  assert.match(view, /aria-labelledby=\{labelId\}/);
});
