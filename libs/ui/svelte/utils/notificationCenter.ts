import type { AppNotification, CommunicationNotification } from "@macui/lib/types";
import { communicationApp } from "./communication";

export type NotificationCenterItem =
  | { key: string; source: "windows"; date: number; notification: AppNotification }
  | { key: string; source: "communication"; date: number; notification: CommunicationNotification };

const DUPLICATE_WINDOW_MS = 15_000;

function normalized(value: string): string {
  return value.normalize("NFKC").replace(/[\u200b-\u200d\ufeff]/g, "").replace(/\s+/g, " ").trim();
}

function nativeTexts(notification: AppNotification): string[] {
  return notification.content.visual.binding.$value.flatMap((entry) => {
    if ("text" in entry) return [entry.text.$value];
    if ("group" in entry) {
      return entry.group.subgroup.flatMap((group) =>
        group.$value.flatMap((child) => "text" in child ? [child.text.$value] : [])
      );
    }
    return [];
  });
}

function messagingIdentity(item: NotificationCenterItem): string | null {
  return item.source === "communication"
    ? item.notification.app
    : communicationApp(`${item.notification.appName} ${item.notification.appUmid}`);
}

function messagingText(item: NotificationCenterItem): string | null {
  const app = messagingIdentity(item);
  if (!app) return null;
  if (item.source === "communication" && item.notification.source === "UnreadHint") return null;
  const texts = item.source === "windows"
    ? nativeTexts(item.notification)
    : [item.notification.title, item.notification.body];
  // Some clients prepend their own application name to a tray balloon.
  const labels: Record<string, RegExp> = {
    WeChat: /^(?:wechat|weixin|微信)$/i,
    WeCom: /^(?:wecom|wxwork|企业微信|企業微信)$/i,
    DingTalk: /^(?:dingtalk|钉钉|釘釘)$/i,
    Feishu: /^(?:feishu|lark|飞书|飛書)$/i,
  };
  const appLabels = labels[app];
  const content = texts.filter((text) => !appLabels?.test(normalized(text)));
  return normalized(content.join(" ")) || null;
}

function duplicates(native: NotificationCenterItem, bridge: NotificationCenterItem): boolean {
  if (native.source !== "windows" || bridge.source !== "communication") return false;
  const app = messagingIdentity(bridge);
  if (!app || app !== messagingIdentity(native)) return false;
  if (Math.abs(native.date - bridge.date) > DUPLICATE_WINDOW_MS) return false;
  if ((bridge.notification.source === "UnreadHint" || bridge.notification.source === "TrayActivity")) return true;
  const text = messagingText(bridge);
  return !!text && text === messagingText(native);
}

/** Preserve full native toasts/reply actions; bridge cards never pretend to be native toasts. */
export function mergeNotificationCenter(
  native: AppNotification[],
  communication: CommunicationNotification[],
): NotificationCenterItem[] {
  const nativeItems: NotificationCenterItem[] = native.map((notification) => ({
    key: `windows:${notification.id}`,
    source: "windows",
    date: notification.date,
    notification,
  }));
  const bridgeItems: NotificationCenterItem[] = communication.map((notification) => ({
    key: `communication:${notification.id}`,
    source: "communication",
    date: notification.date,
    notification,
  }));
  const merged = [
    ...nativeItems,
    ...bridgeItems.filter((item) => !nativeItems.some((native) => duplicates(native, item))),
  ];
  // Prefix keys keep Windows integer IDs independent from bridge UUIDs.
  const unique = new Map<string, NotificationCenterItem>();
  for (const item of merged) {
    if (!Number.isFinite(item.date)) continue;
    const previous = unique.get(item.key);
    if (!previous || previous.date < item.date) unique.set(item.key, item);
  }
  return [...unique.values()].sort((a, b) => b.date - a.date || a.key.localeCompare(b.key));
}

/** A removal, startup fetch, or re-enabling popups must not replay old notifications. */
export class NotificationFlyoutTracker {
  private initialized = new Set<NotificationCenterItem["source"]>();
  private seen = new Set<string>();
  private shown: { app: string; text: string; source: NotificationCenterItem["source"]; date: number }[] = [];

  consume(
    items: NotificationCenterItem[],
    now: number,
    enabled: boolean | null,
    ready: Record<NotificationCenterItem["source"], boolean> = { windows: true, communication: true },
    knownKeys: string[] = [],
  ): NotificationCenterItem | null {
    const initializedBefore = new Set(this.initialized);
    for (const source of ["windows", "communication"] as const) if (ready[source]) this.initialized.add(source);
    const available = items.filter((item) => ready[item.source]);
    const unseen = available.filter((item) => initializedBefore.has(item.source) && !this.seen.has(item.key));
    for (const item of available) {
      if (enabled !== null || !initializedBefore.has(item.source)) this.seen.add(item.key);
    }
    // Hidden cross-source duplicates must remain seen after the native copy is dismissed.
    const visible = new Set(available.map((item) => item.key));
    for (const key of knownKeys) if (!visible.has(key)) this.seen.add(key);
    // Bound history without forgetting still-visible cards.
    if (this.seen.size > 1024) this.seen = new Set([...this.seen].slice(-512).concat(items.map((item) => item.key)));
    this.shown = this.shown.filter((item) => now - item.date <= DUPLICATE_WINDOW_MS);
    if (!enabled) return null;
    for (const item of unseen) {
      if (now - item.date < 0 || now - item.date > 10_000) continue;
      const app = messagingIdentity(item);
      const text = messagingText(item);
      if (app && text) {
        if (
          this.shown.some((previous) =>
            previous.app === app && previous.text === text && previous.source !== item.source
          )
        ) continue;
        this.shown.push({ app, text, source: item.source, date: now });
      }
      return item;
    }
    return null;
  }
}
