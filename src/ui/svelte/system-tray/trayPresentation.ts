import type { SysTrayIcon } from "@macui/lib/types";
import { communicationApp } from "libs/ui/svelte/utils/communication";

export function trayIconKey(item: SysTrayIcon): string {
  return JSON.stringify(item.stable_id);
}

export function trayIconLabel(item: SysTrayIcon): string {
  return item.tooltip.trim() || item.executable?.split(/[\\/]/).pop()?.replace(/\.exe$/i, "") || "";
}

// Task Manager rotates several registered CPU meter icons on one callback
// window. Keep its current native ID for clicks but one tile for that window.
export function trayTileKey(item: SysTrayIcon): string {
  return item.window_handle && /[\\/]system32[\\/]taskmgr\.exe$/i.test(item.executable ?? "")
    ? "task-manager:" + item.window_handle : trayIconKey(item);
}

/** This is the full tray, not the toolbar's messaging shortlist or Windows' pinned subset. */
export function allTrayItems(items: readonly SysTrayIcon[]): SysTrayIcon[] {
  const unique = new Map(items.map((item) => [trayIconKey(item), item]));
  const isMessaging = (item: SysTrayIcon) => !!communicationApp(`${item.executable ?? ""} ${item.tooltip}`);
  const tiles = new Map<string, SysTrayIcon>();
  for (const item of unique.values()) {
    const key = trayTileKey(item);
    const previous = tiles.get(key);
    if (!previous || Number(item.is_visible) > Number(previous.is_visible) ||
      (item.is_visible === previous.is_visible && (item.uid ?? Infinity) < (previous.uid ?? Infinity))) {
      tiles.set(key, item);
    }
  }
  return [...tiles.values()].sort((a, b) =>
    Number(isMessaging(b)) - Number(isMessaging(a)) ||
    (a.executable || trayIconLabel(a)).localeCompare(b.executable || trayIconLabel(b), undefined, { numeric: true }) ||
    trayIconKey(a).localeCompare(trayIconKey(b))
  );
}

/** Limits are based on the monitor, never on the auto-sized popup's own viewport. */
export function trayPanelSize(workWidth: number, workHeight: number, scaleFactor = 1) {
  const scale = Number.isFinite(scaleFactor) && scaleFactor > 0 ? scaleFactor : 1;
  const width = Number.isFinite(workWidth) && workWidth > 0 ? workWidth / scale : 1920;
  const height = Number.isFinite(workHeight) && workHeight > 0 ? workHeight / scale : 1080;
  return {
    width: Math.max(1, Math.min(360, Math.floor(width - 32))),
    maxHeight: Math.max(1, Math.min(600, Math.floor(height - 64))),
  };
}
