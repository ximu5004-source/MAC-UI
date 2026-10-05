import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import type { AppNotification, SysTrayIcon } from "@seelen-ui/lib/types";
import { lazyRune } from "../utils/LazyRune.svelte";
import { communicationApp, hasUnreadHint } from "../utils/communication";

const icons = lazyRune<SysTrayIcon[]>(() => invoke(SeelenCommand.GetSystemTrayIcons));
const notifications = lazyRune<AppNotification[]>(() => invoke(SeelenCommand.GetNotifications));
let ready = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let revision = 0;
let refreshRequest: Promise<void> | null = null;
let iconsSubscription: Promise<unknown> | null = null;
let notificationsReady = $state(false);
let changedIcons = $state<Set<string>>(new Set());
const history = new Map<string, { hash: string | null; times: number[] }>();

function updateIcons(items: SysTrayIcon[]) {
  const current = new Set(items.map((item) => JSON.stringify(item.stable_id)));
  for (const id of history.keys()) if (!current.has(id)) history.delete(id);
  const active = new Set([...changedIcons].filter((id) => current.has(id)));
  const now = Date.now();
  for (const item of items) {
    if (!communicationApp(`${item.executable ?? ""} ${item.tooltip}`)) continue;
    const id = JSON.stringify(item.stable_id);
    const previous = history.get(id);
    const hash = item.icon_image_hash;
    const times = previous?.times.filter((time) => now - time < 6000) ?? [];
    if (previous && hash && previous.hash && previous.hash !== hash) times.push(now);
    if (times.length >= 3) active.add(id);
    history.set(id, { hash, times });
  }
  changedIcons = active;
  icons.value = items;
  ready = true;
  error = null;
  revision += 1;
}

function refreshIcons(): Promise<void> {
  if (refreshRequest) return refreshRequest;
  loading = true;
  error = null;
  refreshRequest = (async () => {
    // Register before fetching, and never overwrite a newer streamed update.
    iconsSubscription ??= subscribe(SeelenEvent.SystemTrayChanged, ({ payload }) => updateIcons(payload))
      .catch((reason) => {
        iconsSubscription = null;
        throw reason;
      });
    await iconsSubscription;
    const requestedAtRevision = revision;
    const items = await invoke(SeelenCommand.GetSystemTrayIcons);
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
void (async () => {
  // Notification access is optional and must never hold up tray enumeration.
  await subscribe(SeelenEvent.Notifications, notifications.setByPayload);
  await notifications.init();
  notificationsReady = true;
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
  acknowledgeActivity(item: SysTrayIcon) {
    const id = JSON.stringify(item.stable_id);
    changedIcons = new Set([...changedIcons].filter((key) => key !== id));
    const previous = history.get(id);
    if (previous) previous.times = [];
  }
}

export const communicationTray = new CommunicationTrayState();
