import assert from "node:assert/strict";
import test from "node:test";
import { didVolumeChange } from "./volumeChange";

test("mute and unmute reveal the volume OSD without changing the level", () => {
  assert.equal(didVolumeChange({ volume: .44, muted: false }, { volume: .44, muted: true }), true);
  assert.equal(didVolumeChange({ volume: .44, muted: true }, { volume: .44, muted: false }), true);
});

test("removed device never reveals an empty OSD", () => {
  assert.equal(didVolumeChange({ volume: .44, muted: false }, undefined), false);
  assert.equal(didVolumeChange({ volume: .44, muted: false }, null), false);
});

test("unchanged visible percentage stays quiet while actual changes reveal the OSD", () => {
  const previous = { volume: .44, muted: false };
  assert.equal(didVolumeChange(previous, { volume: .44001, muted: false }), false);
  assert.equal(didVolumeChange(previous, { volume: .45, muted: false }), true);
});
