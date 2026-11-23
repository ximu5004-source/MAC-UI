import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import { NotificationsMode } from "@macui/lib/types";
import type { AppNotification, CommunicationNotification } from "@macui/lib/types";
import { mergeNotificationCenter } from "../utils/notificationCenter";

let native = $state.raw<AppNotification[]>([]);
let communication = $state.raw<CommunicationNotification[]>([]);
let mode = $state.raw<NotificationsMode>(NotificationsMode.AlarmsOnly);
let nativeReady = $state(false);
let communicationReady = $state(false);
let modeReady = $state(false);
let nativeError = $state(false);
let communicationError = $state(false);
let modeError = $state(false);

async function initializeNative(): Promise<void> {
  let revision = 0;
  try {
    await subscribe(MacUiEvent.Notifications, ({ payload }) => {
      native = payload;
      revision += 1;
      nativeError = false;
      nativeReady = true;
    });
    const requestedAt = revision;
    const fetched = await invoke(MacUiCommand.GetNotifications);
    if (revision === requestedAt) native = fetched;
  } catch (error) {
    // Windows permission is optional for the independent messaging bridge.
    if (!nativeReady) {
      nativeError = true;
      console.warn("Windows notifications are unavailable:", error);
    }
  } finally {
    nativeReady = true;
  }
}

async function initializeCommunication(): Promise<void> {
  let revision = 0;
  try {
    await subscribe(MacUiEvent.CommunicationNotificationsChanged, ({ payload }) => {
      communication = payload;
      revision += 1;
      communicationError = false;
      communicationReady = true;
    });
    const requestedAt = revision;
    const fetched = await invoke(MacUiCommand.GetCommunicationNotifications);
    if (revision === requestedAt) communication = fetched;
  } catch (error) {
    if (!communicationReady) {
      communicationError = true;
      console.warn("Messaging notifications are unavailable:", error);
    }
  } finally {
    communicationReady = true;
  }
}

async function initializeMode(): Promise<void> {
  let revision = 0;
  try {
    await subscribe(MacUiEvent.NotificationsModeChanged, ({ payload }) => {
      mode = payload;
      revision += 1;
      modeError = false;
      modeReady = true;
    });
    const requestedAt = revision;
    const fetched = await invoke(MacUiCommand.GetNotificationsMode);
    if (revision === requestedAt) mode = fetched;
  } catch (error) {
    // Fail closed: do not show automatic popups when DND status is unknown.
    if (!modeReady) {
      modeError = true;
      console.warn("Notification quiet-mode status is unavailable:", error);
    }
  } finally {
    modeReady = true;
  }
}

void initializeNative();
void initializeCommunication();
void initializeMode();

const items = $derived(mergeNotificationCenter(native, communication));
const knownKeys = $derived([
  ...native.map((item) => `windows:${item.id}`),
  ...communication.map((item) => `communication:${item.id}`),
]);

class CommunicationNotificationsState {
  get native() {
    return native;
  }
  get communication() {
    return communication;
  }
  get items() {
    return items;
  }
  get knownKeys() {
    return knownKeys;
  }
  get ready() {
    return nativeReady || communicationReady;
  }
  get sourceReady() {
    return { windows: nativeReady, communication: communicationReady };
  }
  get focusAssistMode() {
    return mode;
  }
  get modePending() {
    return !modeReady;
  }
  get modeAvailable() {
    return modeReady && !modeError;
  }
  get nativeUnavailable() {
    return nativeReady && nativeError;
  }
  get communicationUnavailable() {
    return communicationReady && communicationError;
  }
}

export const communicationNotifications = new CommunicationNotificationsState();
