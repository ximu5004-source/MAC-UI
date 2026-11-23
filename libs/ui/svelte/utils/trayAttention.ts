/** Pure state machine driven only by published tray pixels and notifications. */
export interface TrayObservation {
  id: string;
  hash: string | null;
  notificationKeys: readonly string[];
}

interface Observation {
  hash: string | null;
  changes: number[];
  lastChange: number;
  activityUntil: number;
  notificationUntil: number;
  notificationKeys: Set<string>;
  acknowledgedBurst: boolean;
}

export const TRAY_ACTIVITY_WINDOW_MS = 6000;
export const TRAY_NOTIFICATION_PULSE_MS = 15000;

export class TrayAttention {
  private observations = new Map<string, Observation>();

  observe(items: readonly TrayObservation[], now: number, baselineNotifications = false) {
    const present = new Set(items.map((item) => item.id));
    for (const id of this.observations.keys()) if (!present.has(id)) this.observations.delete(id);
    for (const item of items) {
      const previous = this.observations.get(item.id);
      if (!previous) {
        // Existing notifications and the initial image are a baseline, not new messages.
        this.observations.set(item.id, {
          hash: item.hash, changes: [], lastChange: -Infinity,
          activityUntil: 0, notificationUntil: 0,
          notificationKeys: new Set(item.notificationKeys), acknowledgedBurst: false,
        });
        continue;
      }
      if (now - previous.lastChange >= TRAY_ACTIVITY_WINDOW_MS) previous.acknowledgedBurst = false;
      previous.changes = previous.changes.filter((time) => now - time < TRAY_ACTIVITY_WINDOW_MS);
      if (item.hash && previous.hash && item.hash !== previous.hash) {
        previous.changes.push(now);
        previous.lastChange = now;
        // A repeated pixel change is activity, never an invented unread count/body.
        if (previous.changes.length >= 3 && !previous.acknowledgedBurst) {
          previous.activityUntil = now + TRAY_ACTIVITY_WINDOW_MS;
        }
      }
      previous.hash = item.hash;
      if (!baselineNotifications && item.notificationKeys.some((key) => !previous.notificationKeys.has(key))) {
        previous.notificationUntil = now + TRAY_NOTIFICATION_PULSE_MS;
      }
      if (item.notificationKeys.length === 0) previous.notificationUntil = 0;
      previous.notificationKeys = new Set(item.notificationKeys);
    }
  }

  acknowledge(id: string) {
    const item = this.observations.get(id);
    if (!item) return;
    item.activityUntil = 0;
    item.notificationUntil = 0;
    // Do not restart a continuous native flashing burst immediately after click.
    // It becomes eligible again after quiet, or on a genuinely new notification.
    item.acknowledgedBurst = true;
  }

  snapshot(now: number) {
    const active = new Set<string>();
    const flashing = new Set<string>();
    let nextDeadline: number | null = null;
    for (const [id, item] of this.observations) {
      if (item.activityUntil > now) active.add(id);
      if (item.activityUntil > now || item.notificationUntil > now) flashing.add(id);
      for (const deadline of [item.activityUntil, item.notificationUntil]) {
        if (deadline > now && (nextDeadline === null || deadline < nextDeadline)) nextDeadline = deadline;
      }
    }
    return { active, flashing, nextDeadline };
  }
}
