import assert from "node:assert/strict";
import { test } from "node:test";
import type { MediaPlayer } from "@macui/lib/types";
import { mediaPopupViewFromArgs, selectNowPlayingSession } from "./model.ts";

function player(id: string, playing = false, isDefault = false): MediaPlayer {
  return {
    umid: id,
    title: id,
    author: "Artist",
    owner: { name: id },
    thumbnail: null,
    playing,
    default: isDefault,
    timeline: { start: 0, end: 180_000_000_000, position: 0, minSeek: 0, maxSeek: 180_000_000_000, lastUpdatedTime: 0 },
  };
}

test("Now Playing handles an empty session list", () => {
  assert.equal(selectNowPlayingSession([]), undefined);
});

test("Now Playing respects the OS default even when paused", () => {
  const paused = player("paused", false, true);
  assert.equal(selectNowPlayingSession([player("active", true), paused]), paused);
});

test("Now Playing falls back to playing session, then the first available session", () => {
  const paused = player("paused");
  const playing = player("active", true);
  assert.equal(selectNowPlayingSession([paused, playing]), playing);
  assert.equal(selectNowPlayingSession([paused]), paused);
});

test("Now Playing selection never reorders the stored session list", () => {
  const sessions = [player("first"), player("second", true)];
  const before = [...sessions];
  selectNowPlayingSession(sessions);
  assert.deepEqual(sessions, before);
});

test("A normal media-popup trigger always opens the original volume view", () => {
  for (const args of [undefined, null, {}, { view: "mixer" }, { view: "bad" }, { view: true }]) {
    assert.equal(mediaPopupViewFromArgs(args), "main");
  }
});

test("Only the explicit Now Playing trigger opens playback controls", () => {
  assert.equal(mediaPopupViewFromArgs({ view: "now-playing" }), "now-playing");
});
