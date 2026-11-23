import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import type { MediaPlayer } from "@macui/lib/types";
import { type LazyRune, lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";
import { selectNowPlayingSession } from "../../../media-popup/model.ts";

let sessions = $state.raw<LazyRune<MediaPlayer[]> | null>(null);
let ready = $state(false);
let unavailable = $state(false);
let consumers = 0;
let generation = 0;
let unlisten: (() => void) | undefined;

async function initialize(value: LazyRune<MediaPlayer[]>, version: number) {
  try {
    // Subscribe before fetching. LazyRune keeps an event newer than the initial fetch.
    const release = await subscribe(MacUiEvent.MediaSessions, (event) => {
      if (version !== generation) return;
      value.setByPayload(event);
      ready = true;
      unavailable = false;
    });
    if (version !== generation) {
      release();
      return;
    }
    unlisten = release;
    await value.init();
    if (version === generation) ready = true;
  } catch (error) {
    if (version !== generation) return;
    console.error("Unable to initialize Now Playing", error);
    if (!value.isInitialized()) {
      value.value = [];
      ready = true;
      unavailable = true;
    }
  }
}

class NowPlayingState {
  get session() {
    return selectNowPlayingSession(ready && sessions ? sessions.value : []);
  }

  get unavailable() {
    return unavailable;
  }

  /** Start only while the module is mounted; a drag overlay shares the subscription. */
  retain(): () => void {
    if (consumers++ === 0) {
      const value = lazyRune<MediaPlayer[]>(() => invoke(MacUiCommand.GetMediaSessions));
      sessions = value;
      ready = false;
      unavailable = false;
      void initialize(value, ++generation);
    }

    let released = false;
    return () => {
      if (released) return;
      released = true;
      if (--consumers === 0) {
        generation++;
        unlisten?.();
        unlisten = undefined;
        sessions = null;
        ready = false;
        unavailable = false;
      }
    };
  }
}

export const nowPlayingState = new NowPlayingState();
