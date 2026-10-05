import type { StartMenuItem } from "@seelen-ui/lib/types";

export interface FavAppItem { type: "app"; itemId: string }
export interface FavFolderItem {
  type: "folder";
  itemId: string;
  name: string;
  itemIds: string[];
  /** Stable Windows folder identity, independent of the user's chosen display name. */
  sourceKey?: string;
}
export type FavPinnedItem = FavAppItem | FavFolderItem;
export interface LaunchpadLayout { version: 1; items: FavPinnedItem[] }

export const getItemId = (item: Pick<StartMenuItem, "umid" | "path">) => item.umid || item.path.toLowerCase();
export const isApplicationItem = (item: Pick<StartMenuItem, "umid" | "path">) =>
  !!item.umid || /\.(lnk|url|exe|appref-ms)$/i.test(item.path || "");
const key = (id: string) => id.toLocaleLowerCase("en-US");

/** Validate persisted/legacy layouts without losing single-item or empty folders. */
export function normalizeLayout(raw: unknown): FavPinnedItem[] {
  if (!Array.isArray(raw)) return [];
  const seenApps = new Set<string>();
  const seenFolders = new Set<string>();
  const output: FavPinnedItem[] = [];
  for (const item of raw) {
    if (!item || typeof item.itemId !== "string" || !item.itemId.trim()) continue;
    if (item.type === "app") {
      if (seenApps.has(key(item.itemId))) continue;
      seenApps.add(key(item.itemId));
      output.push({ type: "app", itemId: item.itemId });
    } else if (item.type === "folder" && Array.isArray(item.itemIds)) {
      if (seenFolders.has(item.itemId)) continue;
      seenFolders.add(item.itemId);
      const ids = item.itemIds.filter((id: unknown): id is string => {
        if (typeof id !== "string" || !id.trim() || seenApps.has(key(id))) return false;
        seenApps.add(key(id));
        return true;
      });
      output.push({
        type: "folder", itemId: item.itemId,
        name: typeof item.name === "string" ? item.name : "", itemIds: ids,
        ...(typeof item.sourceKey === "string" ? { sourceKey: item.sourceKey } : {}),
      });
    }
  }
  return output;
}

export function startMenuCatalog(items: StartMenuItem[]): StartMenuItem[] {
  const seen = new Set<string>();
  return items.filter((item) => {
    if (!isApplicationItem(item)) return false;
    const id = key(getItemId(item));
    if (!id || seen.has(id)) return false;
    seen.add(id);
    return true;
  });
}

/** Idempotent merge: preserve manual order, folder names and existing memberships. */
export function importStartMenu(existing: FavPinnedItem[], catalog: StartMenuItem[]) {
  const items = normalizeLayout(existing);
  const occupied = new Set(items.flatMap(i => i.type === "app" ? [key(i.itemId)] : i.itemIds.map(key)));
  let addedApps = 0, addedFolders = 0;
  const sorted = startMenuCatalog(catalog).toSorted((a, b) => a.display_name.localeCompare(b.display_name));
  for (const app of sorted) {
    const id = getItemId(app);
    if (occupied.has(key(id))) continue;
    occupied.add(key(id));
    addedApps++;
    const segments = (app.start_menu_folder || []).filter(s => s && s !== "." && s !== "..");
    if (!segments.length) {
      items.push({ type: "app", itemId: id });
      continue;
    }
    const sourceKey = segments.map(key).join("/");
    let folder = items.find((i): i is FavFolderItem => i.type === "folder" && i.sourceKey === sourceKey);
    if (!folder) {
      const base = `start-folder:${encodeURIComponent(sourceKey)}`;
      let folderId = base;
      let suffix = 1;
      while (items.some(i => i.itemId === folderId)) folderId = `${base}:${suffix++}`;
      folder = { type: "folder", itemId: folderId, name: segments.join(" / "), itemIds: [], sourceKey };
      items.push(folder);
      addedFolders++;
    }
    folder.itemIds.push(id);
  }
  return { items, addedApps, addedFolders };
}

export function removeApp(items: FavPinnedItem[], id: string): FavPinnedItem[] {
  return items.filter(i => i.type !== "app" || key(i.itemId) !== key(id)).map(i =>
    i.type === "folder" ? { ...i, itemIds: i.itemIds.filter(child => key(child) !== key(id)) } : i
  );
}

export function moveIntoFolder(items: FavPinnedItem[], id: string, folderId: string): FavPinnedItem[] {
  if (!items.some(i => i.type === "folder" && i.itemId === folderId)) return items;
  return removeApp(items, id).map(i => i.type === "folder" && i.itemId === folderId
    ? { ...i, itemIds: [...i.itemIds, id] } : i);
}

export function disbandFolder(items: FavPinnedItem[], folderId: string): FavPinnedItem[] {
  return normalizeLayout(items.flatMap(i => i.type === "folder" && i.itemId === folderId
    ? i.itemIds.map(itemId => ({ type: "app" as const, itemId })) : [i]));
}

export function moveRootItem(items: FavPinnedItem[], id: string, offset: number): FavPinnedItem[] {
  const from = items.findIndex(i => i.itemId === id);
  if (from < 0) return items;
  const to = Math.max(0, Math.min(items.length - 1, from + offset));
  const result = [...items];
  result.splice(to, 0, result.splice(from, 1)[0]!);
  return result;
}

export function pageGeometry(width: number, height: number, preferredIconSize?: 48 | 64) {
  // Measure the application viewport, including its 40px page controls.
  // A normal cell reserves 64px artwork + 8px spacing + two 20px label
  // lines + 8px padding. Compact cells keep the same legible label size.
  const availableWidth = Number.isFinite(width) && width > 0 ? width : 320;
  const availableHeight = Number.isFinite(height) && height > 0 ? height : 200;
  const iconSize = preferredIconSize ?? (availableWidth < 600 || availableHeight < 360 ? 48 : 64);
  const compact = iconSize === 48;
  const columnGap = compact ? 16 : 24, rowGap = compact ? 12 : 16;
  const cellWidth = compact ? 88 : 104, cellHeight = iconSize + 56;
  const columns = Math.max(1, Math.min(8, Math.floor((availableWidth - 16 + columnGap) / (cellWidth + columnGap))));
  const rows = Math.max(1, Math.min(5, Math.floor((availableHeight - 56 + rowGap) / (cellHeight + rowGap))));
  return { columns, rows, pageSize: columns * rows, iconSize };
}

export function pageSlice<T>(items: T[], requested: number, pageSize: number) {
  const size = Math.max(1, Math.floor(pageSize));
  const count = Math.max(1, Math.ceil(items.length / size));
  const page = Math.max(0, Math.min(count - 1, Math.floor(requested) || 0));
  return { items: items.slice(page * size, (page + 1) * size), page, count, offset: page * size };
}
