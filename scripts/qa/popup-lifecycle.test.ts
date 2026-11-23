import assert from "node:assert/strict";
import { test } from "node:test";
import {
  PopupIntent,
  popupOwnsFocus,
  popupToggleSource,
  shouldDismissPopupAfterFocusLoss,
} from "../../libs/core/src/state/widget/popup_lifecycle.ts";

test("same toolbar source closes a pending popup before preparation finishes", () => {
  const state = new PopupIntent();
  const first = state.begin("volume:monitor1");
  assert.equal(state.begin("volume:monitor1").close, true);
  assert.equal(state.isCurrent(first.token), false);
  assert.equal(state.isOpen, false);
});

test("external dismissal invalidates in-flight show and later clicks reopen", () => {
  const state = new PopupIntent();
  const pending = state.begin("network");
  state.dismiss();
  assert.equal(state.isCurrent(pending.token), false);
  const next = state.begin("network");
  assert.equal(next.close, false);
  assert.equal(state.isCurrent(next.token), true);
});

test("hidden preparation survives old foreground title updates but cancels a real focus move", () => {
  const state = new PopupIntent();
  const first = state.begin("volume");
  assert.equal(shouldDismissPopupAfterFocusLoss(false, 42, 42), false);
  assert.equal(state.isCurrent(first.token), true);
  assert.equal(shouldDismissPopupAfterFocusLoss(false, undefined, 42), false);
  if (shouldDismissPopupAfterFocusLoss(false, 42, 99)) state.dismiss();
  assert.equal(state.isCurrent(first.token), false);
  assert.equal(shouldDismissPopupAfterFocusLoss(true, 42, 42), true);
});

test("different controls may open different views and hover triggers never toggle", () => {
  const state = new PopupIntent();
  const volume = state.begin("volume");
  const music = state.begin("music");
  assert.equal(music.close, false);
  assert.equal(state.isCurrent(volume.token), false);
  assert.equal(state.begin(null).close, false);
  assert.equal(state.begin(null).close, false);
});

test("native owned dialogs count as inside; unknown or outside windows do not", () => {
  assert(popupOwnsFocus(10, { hwnd: 10 }));
  assert(popupOwnsFocus(10, { hwnd: 12, ownerHwnd: 10 }));
  assert.equal(popupOwnsFocus(10, { hwnd: 12, ownerHwnd: 2 }), false);
  assert.equal(popupOwnsFocus(0, { hwnd: 0 }), false);
  assert.equal(popupToggleSource({ view: "main" }), null);
  assert.equal(popupToggleSource({ __macuiPopupToggleSource: "volume" }), "volume");
  assert.equal(popupToggleSource({ __macuiPopupToggleSource: "x".repeat(257) }), null);
});
