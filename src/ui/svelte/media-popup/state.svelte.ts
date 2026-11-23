import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe } from "@macui/lib";
import type { MediaDevice, MediaPlayer, WidgetTriggerPayload } from "@macui/lib/types";
import { tick } from "svelte";
import { locale } from "./i18n/index.ts";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";
import { mediaPopupViewFromArgs, type MediaPopupView } from "./model.ts";

const settings = lazyRune(() => Settings.getAsync());
await Settings.onChange((s) => (settings.value = s));
await settings.init();

$effect.root(() => {
  $effect(() => {
    locale.set(settings.value.language);
  });
});

let mediaDevices = lazyRune(() => invoke(MacUiCommand.GetMediaDevices));
await subscribe(MacUiEvent.MediaDevices, mediaDevices.setByPayload);

let mediaSessions = lazyRune(() => invoke(MacUiCommand.GetMediaSessions));
await subscribe(MacUiEvent.MediaSessions, mediaSessions.setByPayload);

await Promise.all([mediaDevices.init(), mediaSessions.init()]);

let currentView = $state<MediaPopupView>("main");
let selectedDeviceId = $state<string | null>(null);

class State {
  async prepareShow({ customArgs }: WidgetTriggerPayload): Promise<void> {
    currentView = mediaPopupViewFromArgs(customArgs);
    selectedDeviceId = null;
    // Keep the selected view during focus loss/hide animations. The next trigger
    // explicitly selects its own view, and the preset waits for this DOM commit.
    await tick();
  }

  get inputs(): MediaDevice[] {
    return mediaDevices.value[0];
  }

  get outputs(): MediaDevice[] {
    return mediaDevices.value[1];
  }

  get sessions(): MediaPlayer[] {
    return mediaSessions.value;
  }

  get defaultInput(): MediaDevice | undefined {
    return this.inputs.find((d) => d.isDefaultMultimedia);
  }

  get defaultOutput(): MediaDevice | undefined {
    return this.outputs.find((d) => d.isDefaultMultimedia);
  }

  get view() {
    return currentView;
  }

  set view(value: MediaPopupView) {
    currentView = value;
  }

  get selectedDeviceId() {
    return selectedDeviceId;
  }

  set selectedDeviceId(value: string | null) {
    selectedDeviceId = value;
  }

  get selectedDevice(): MediaDevice | undefined {
    if (!selectedDeviceId) return undefined;
    return (
      this.inputs.find((d) => d.id === selectedDeviceId) ||
      this.outputs.find((d) => d.id === selectedDeviceId)
    );
  }
}

export const globalState = new State();
