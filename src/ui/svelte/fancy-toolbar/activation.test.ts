import assert from "node:assert/strict";
import { test } from "node:test";
import { isPrimaryToolbarClick, isToolbarActivationKey } from "./activation.ts";

test("toolbar activation accepts only primary clicks not consumed by dragging", () => {
  assert.equal(isPrimaryToolbarClick({ button: 0, defaultPrevented: false }), true);
  for (const button of [1, 2, 3, 4]) {
    assert.equal(isPrimaryToolbarClick({ button, defaultPrevented: false }), false);
  }
  assert.equal(isPrimaryToolbarClick({ button: 0, defaultPrevented: true }), false);
  assert.equal(isPrimaryToolbarClick({ button: 0, defaultPrevented: false }, true), false);
});

test("keyboard activation never turns menu/shortcut keys into left clicks", () => {
  const plain = { repeat: false, altKey: false, ctrlKey: false, metaKey: false };
  for (const key of ["Enter", " "]) {
    assert.equal(isToolbarActivationKey({ ...plain, key }), true);
    assert.equal(isToolbarActivationKey({ ...plain, key, repeat: true }), false);
    assert.equal(isToolbarActivationKey({ ...plain, key, ctrlKey: true }), false);
  }
  for (const key of ["ContextMenu", "F10", "Escape", "ArrowRight"]) {
    assert.equal(isToolbarActivationKey({ ...plain, key }), false);
  }
});
