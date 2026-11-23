import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import type { AppNotification, SysTrayIcon } from "@macui/lib/types";
import { emit, listen } from "@tauri-apps/api/event";
import { lazyRune } from "../utils/LazyRune.svelte";
import { communicationApp, hasUnreadHint } from "../utils/communication";
import { TrayAttention } from "../utils/trayAttention";

const icons = lazyRune<SysTrayIcon[]>(() => invoke(MacUiCommand.GetSystemTrayIcons));
const notifications = lazyRune<AppNotification[]>(() => invoke(MacUiCommand.GetNotifications));
let ready = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let revision = 0;
let refreshRequest: Promise<void> | null = null;
let iconsSubscription: Promise<unknown> | null = null;
let notificationsReady = $state(false);
let changedIcons = $state<Set<string>>(new Set());
let flashingIcons = $state<Set<string>>(new Set());
const attention = new TrayAttention();
let attentionTimer: ReturnType<typeof setTimeout> | undefined;
const ATTENTION_ACKNOWLEDGED = "macui:tray-attention-acknowledged";

function refreshAttention() {
  if (attentionTimer !== undefined) clearTimeout(attentionTimer);
  const now = Date.now();
  const snapshot = attention.snapshot(now);
  changedIcons = snapshot.active;
  flashingIcons = snapshot.flashing;
  attentionTimer = snapshot.nextDeadline === null ? undefined
    : setTimeout(refreshAttention, Math.max(1, snapshot.nextDeadline - now));
}

function observeAttention(baselineNotifications = false) {
  if (!ready) return;
  attention.observe(icons.value.filter((item) => communicationApp(`${item.executable ?? ""} ${item.tooltip}`)).map((item) => {
    const name = communicationApp(`${item.executable ?? ""} ${item.tooltip}`);
    const native = notificationsReady ? notifications.value.filter((notification) =>
      communicationApp(`${notification.appName} ${notification.appUmid}`) === name
    ).map((notification) => `toast:${notification.appUmid}:${notification.id}:${notification.date}`) : [];
    return {
      id: JSON.stringify(item.stable_id),
      hash: item.icon_image_hash,
      notificationKeys: [
        ...(item.notification ? [`balloon:${item.notification_revision}`] : []),
        ...(hasUnreadHint(item.tooltip) ? [`unread:${item.tooltip}`] : []),
        ...native,
      ],
    };
  }), Date.now(), baselineNotifications);
  refreshAttention();
}

function updateIcons(items: SysTrayIcon[]) {
  icons.value = items;
  ready = true;
  error = null;
  revision += 1;
  observeAttention();
}

function refreshIcons(): Promise<void> {
  if (refreshRequest) return refreshRequest;
  loading = true;
  error = null;
  refreshRequest = (async () => {
    // Register before fetching, and never overwrite a newer streamed update.
    iconsSubscription ??= subscribe(MacUiEvent.SystemTrayChanged, ({ payload }) => updateIcons(payload))
      .catch((reason) => {
        iconsSubscription = null;
        throw reason;
      });
    await iconsSubscription;
    const requestedAtRevision = revision;
    const items = await invoke(MacUiCommand.GetSystemTrayIcons);
    if (revision === requestedAtRevision) updateIcons(items);
  })().catch((reason) => {
    console.error(reason);
    error = String(reason);
  }).finally(() => {
    loading = false;
    refreshRequest = null;
  });
  return refreshRequest;
}

void refreshIcons();
// The popup and each monitor's toolbar are different webviews. A click in one
// acknowledges this local visual alert in all of them, not the client's chats.
void listen<string>(ATTENTION_ACKNOWLEDGED, ({ payload }) => {
  if (typeof payload !== "string" || payload.length > 256) return;
  attention.acknowledge(payload);
  refreshAttention();
}).catch(console.error);
void (async () => {
  // Notification access is optional and must never hold up tray enumeration.
  await subscribe(MacUiEvent.Notifications, ({ payload }) => {
    const baseline = !notificationsReady;
    notifications.value = payload;
    notificationsReady = true;
    observeAttention(baseline);
  });
  await notifications.init();
  if (!notificationsReady) {
    notificationsReady = true;
    observeAttention(true);
  }
})().catch(console.error);

class CommunicationTrayState {
  get loading() {
    return loading;
  }
  get error() {
    return error;
  }
  refresh() {
    return refreshIcons();
  }
  get items() {
    return ready ? icons.value : [];
  }
  get communicationItems() {
    return this.items.filter((item) => this.name(item) !== null);
  }
  name(item: SysTrayIcon) {
    return communicationApp(`${item.executable ?? ""} ${item.tooltip}`);
  }
  nativeCount(item: SysTrayIcon) {
    if (!notificationsReady) return 0;
    const name = this.name(item);
    if (!name) return 0;
    return notifications.value.filter((n) => communicationApp(`${n.appName} ${n.appUmid}`) === name).length;
  }
  hasNotification(item: SysTrayIcon) {
    return !!item.notification || this.nativeCount(item) > 0 || hasUnreadHint(item.tooltip);
  }
  hasActivity(item: SysTrayIcon) {
    return changedIcons.has(JSON.stringify(item.stable_id));
  }
  isFlashing(item: SysTrayIcon) {
    return flashingIcons.has(JSON.stringify(item.stable_id));
  }
  acknowledgeActivity(item: SysTrayIcon) {
    const id = JSON.stringify(item.stable_id);
    attention.acknowledge(id);
    refreshAttention();
    void emit(ATTENTION_ACKNOWLEDGED, id).catch(console.error);
  }
}

export const communicationTray = new CommunicationTrayState();
