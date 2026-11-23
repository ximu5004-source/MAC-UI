import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import { lazyRune } from "libs/ui/svelte/utils";
import { communicationNotifications } from "libs/ui/svelte/runes/CommunicationNotifications.svelte";

let shortcutsPaused = $state<boolean | null>(null);
subscribe(MacUiEvent.ShortcutsPaused, (e) => {
  shortcutsPaused = e.payload;
});

let mediaDevices = lazyRune(() => invoke(MacUiCommand.GetMediaDevices));
subscribe(MacUiEvent.MediaDevices, mediaDevices.setByPayload);

let mediaPlaying = lazyRune(() => invoke(MacUiCommand.GetMediaSessions));
subscribe(MacUiEvent.MediaSessions, mediaPlaying.setByPayload);

let brightness = lazyRune(() => invoke(MacUiCommand.GetAllMonitorsBrightness));
subscribe(MacUiEvent.SystemMonitorsBrightnessChanged, brightness.setByPayload);

let workspaces = lazyRune(() => invoke(MacUiCommand.StateGetVirtualDesktops));
subscribe(MacUiEvent.VirtualDesktopsChanged, workspaces.setByPayload);

const snapshots = await Promise.allSettled([
  mediaDevices.init(),
  mediaPlaying.init(),
  brightness.init(),
  workspaces.init(),
]);
for (const result of snapshots) {
  if (result.status === "rejected") console.warn("Flyout data source unavailable", result.reason);
}

class State {
  get mediaInputs() {
    return mediaDevices.snapshot?.[0] ?? [];
  }
  get mediaOutputs() {
    return mediaDevices.snapshot?.[1] ?? [];
  }
  get mediaPlaying() {
    return mediaPlaying.snapshot ?? [];
  }
  get brightness() {
    return brightness.snapshot?.[0] ?? null;
  }
  get workspaces() {
    return workspaces.snapshot;
  }
  get notifications() {
    return communicationNotifications.items;
  }
  get notificationsMode() {
    return communicationNotifications.focusAssistMode;
  }
  get notificationsModePending() {
    return communicationNotifications.modePending;
  }
  get notificationSourcesReady() {
    return communicationNotifications.sourceReady;
  }
  get notificationKnownKeys() {
    return communicationNotifications.knownKeys;
  }
  get shortcutsPaused() {
    return shortcutsPaused;
  }
}

export const state = new State();
