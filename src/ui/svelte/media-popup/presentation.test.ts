import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const source = (name: string) => readFileSync(new URL(name, import.meta.url), "utf8");
const state = source("./state.svelte.ts");
const entry = source("./index.ts");
const widget = source("../../../../libs/core/src/state/widget/mod.ts");
const popupPreset = widget.slice(widget.indexOf("private applyPopupPreset("), widget.indexOf("private async hideOnFocusLoss("));

test("media entry wires view preparation into the awaited Popup before-show hook", () => {
  assert.match(entry, /beforePopupShow:\s*\(payload\)\s*=>\s*globalState\.prepareShow\(payload\)/);
  assert.match(widget, /this\.applyPopupPreset\(options\)/);
  assert.match(state, /async prepareShow\(\{ customArgs \}: WidgetTriggerPayload\): Promise<void>/);
  const prepare = state.slice(state.indexOf("async prepareShow("), state.indexOf("get inputs("));
  assert.ok(prepare.indexOf("currentView = mediaPopupViewFromArgs(customArgs)") < prepare.indexOf("await tick()"));
  assert.ok(prepare.indexOf("selectedDeviceId = null") < prepare.indexOf("await tick()"));
});

test("focus loss keeps the selected media view during native hide animations", () => {
  assert.doesNotMatch(state, /onFocusChanged|onTrigger\(/);
  assert.equal([...state.matchAll(/currentView\s*=\s*["']main["']/g)].length, 0);
});

test("Popup waits for target DOM and measures it before positioning or revealing the native window", () => {
  const steps = [
    "await beforeShow(payload)",
    "await this.executeAutoSize()",
    "await this.adjustAndSetPosition(",
    "await this.show()",
    "await this.focus()",
  ].map((step) => popupPreset.indexOf(step));
  assert.ok(steps.every((index) => index >= 0), "The before-show preparation chain is missing a step");
  assert.deepEqual(steps, [...steps].sort((a, b) => a - b), "The old frame must never be shown before the target DOM is measured");
  assert.ok(popupPreset.lastIndexOf("await this.executeAutoSize()") > popupPreset.indexOf("await this.focus()"), "Retain the final stale-frame correction after focus");
});

// Superseding and dismissed preparations are exercised behaviorally in
// scripts/qa/popup-lifecycle.test.ts rather than tied to the token's spelling.
