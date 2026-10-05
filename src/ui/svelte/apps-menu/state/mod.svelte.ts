import { invoke, SeelenCommand, SeelenEvent, Settings, subscribe } from "@seelen-ui/lib";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { StartMenuItem } from "@seelen-ui/lib/types";
import { lazyRune, persistentRune } from "libs/ui/svelte/utils";
import { StartDisplayMode, StartView } from "../constants";
import { foldersAsStartMenuItems } from "./knownFolders.svelte";
import { locale } from "../i18n/index.ts";
import {
  disbandFolder, getItemId, importStartMenu, moveIntoFolder, moveRootItem,
  normalizeLayout, removeApp, startMenuCatalog,
  type FavFolderItem, type FavPinnedItem, type LaunchpadLayout,
} from "../launchpad";
export type { FavAppItem, FavFolderItem, FavPinnedItem } from "../launchpad";

const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));
const user = lazyRune(() => invoke(SeelenCommand.GetUser));
subscribe(SeelenEvent.UserChanged, user.setByPayload);
const monitors = lazyRune(() => invoke(SeelenCommand.SystemGetMonitors));
subscribe(SeelenEvent.SystemMonitorsChanged, monitors.setByPayload);
const startMenuItems = lazyRune(() => invoke(SeelenCommand.GetStartMenuItems));
subscribe(SeelenEvent.StartMenuItemsChanged, startMenuItems.setByPayload);
await Promise.all([settings.init(), user.init(), monitors.init(), startMenuItems.init()]);

$effect.root(() => { $effect(() => { locale.set(settings.value.language); }); });

const STORAGE_FILE = "launchpad-v1.json";
let pinnedItems = $state<FavPinnedItem[]>([]);
let saveState = $state<"saved" | "saving" | "error">("saved");
let importing = $state(false);
let importResult = $state<{ addedApps: number; addedFolders: number } | null>(null);
let importFailed = $state(false);
let writeQueue: Promise<void> = Promise.resolve();
let revision = 0;

function saveItems(items: FavPinnedItem[]): Promise<void> {
  pinnedItems = normalizeLayout(items);
  const content = JSON.stringify({ version: 1, items: pinnedItems } satisfies LaunchpadLayout);
  const currentRevision = ++revision;
  saveState = "saving";
  const save = writeQueue.catch(() => {}).then(() => invoke(SeelenCommand.WriteFile, { filename: STORAGE_FILE, content }));
  writeQueue = save;
  void save.then(() => {
    if (revision === currentRevision) saveState = "saved";
  }, (error) => {
    console.error("Launchpad layout save failed:", error);
    if (revision === currentRevision) saveState = "error";
  });
  return save;
}

// Use a new file so the previous favorites layout is always available for rollback.
let stored: string | null = null;
try { stored = await invoke(SeelenCommand.ReadFile, { filename: STORAGE_FILE }); } catch { /* First launch. */ }
if (stored !== null) {
  try {
    const parsed = JSON.parse(stored);
    if (parsed?.version !== 1 || !Array.isArray(parsed.items)) throw new Error("Invalid Launchpad layout");
    pinnedItems = normalizeLayout(parsed.items);
  } catch {
    // Preserve corrupt/unknown data before recovery; if that fails, don't overwrite it.
    await invoke(SeelenCommand.WriteFile, { filename: `launchpad-recovery-${Date.now()}.json`, content: stored });
    stored = null;
  }
}
if (stored === null) {
  let previous: FavPinnedItem[] = [];
  try {
    previous = normalizeLayout(JSON.parse(await invoke(SeelenCommand.ReadFile, { filename: "favorites.json" })));
  } catch {
    try {
      const native = await invoke(SeelenCommand.GetNativeStartMenu);
      previous = normalizeLayout(native.pinnedList.map((entry) => {
        const item = entry as unknown as Record<string, string>;
        const path = item.desktopAppLink;
        const matched = path && startMenuItems.value.find(i => i.path.toLowerCase() === path.toLowerCase());
        return { type: "app", itemId: matched ? getItemId(matched) : item.packagedAppId || item.destopAppId || path };
      }));
    } catch (error) { console.warn("Native pins unavailable; using Start Menu catalog", error); }
  }
  await saveItems(importStartMenu(previous, startMenuItems.value).items).catch(() => {});
}

const displayMode = await persistentRune("launchpad-display-mode-v1", StartDisplayMode.Fullscreen);
let view = $state(StartView.Favorites);
let version = $state(0);
let page = $state(0);
let columns = $state(7);
let pageSize = $state(35);
let openFolderId = $state<string | null>(null);
let wallpaper = $state("");

class State {
  get wallpaper() { return wallpaper; }
  async refreshWallpaper() {
    try {
      const path = await invoke(SeelenCommand.GetNativeShellWallpaper);
      wallpaper = path ? convertFileSrc(path) : "";
    } catch { wallpaper = ""; }
  }
  get view() { return view; }
  set view(value: StartView) { view = value; }
  get version() { return version; }
  set version(value: number) { version = value; }
  get page() { return page; }
  set page(value: number) { page = value; }
  get columns() { return columns; }
  set columns(value: number) { columns = value; }
  get pageSize() { return pageSize; }
  set pageSize(value: number) { pageSize = value; }
  get openFolderId() { return openFolderId; }
  set openFolderId(value: string | null) { openFolderId = value; }
  getItemId = getItemId;

  get user() { return user.value; }
  get monitors() { return monitors.value; }
  get allItems() { return startMenuCatalog(startMenuItems.value); }
  get pinnedItems() { return pinnedItems; }
  set pinnedItems(value: FavPinnedItem[]) { void saveItems(value).catch(() => {}); }
  get saveState() { return saveState; }
  get importing() { return importing; }
  get importResult() { return importResult; }
  get importFailed() { return importFailed; }
  get displayMode() { return displayMode.value; }
  set displayMode(value: StartDisplayMode) { displayMode.value = value; }
  get folders() { return pinnedItems.filter((i): i is FavFolderItem => i.type === "folder"); }

  retrySave() { void saveItems(pinnedItems).catch(() => {}); }

  async importWindowsStartMenu() {
    if (importing) return;
    importing = true;
    importFailed = false;
    importResult = null;
    try {
      startMenuItems.value = await invoke(SeelenCommand.GetStartMenuItems);
      const result = importStartMenu(pinnedItems, startMenuItems.value);
      await saveItems(result.items);
      importResult = { addedApps: result.addedApps, addedFolders: result.addedFolders };
      this.view = StartView.Favorites;
    } catch (error) {
      importFailed = true;
      console.error("Start Menu import failed:", error);
    } finally { importing = false; }
  }

  isPinned(item: StartMenuItem) {
    const id = getItemId(item).toLowerCase();
    return pinnedItems.some(i => i.type === "app" ? i.itemId.toLowerCase() === id : i.itemIds.some(child => child.toLowerCase() === id));
  }
  togglePin(item: StartMenuItem) {
    const id = getItemId(item);
    this.pinnedItems = this.isPinned(item) ? removeApp(pinnedItems, id) : [...pinnedItems, { type: "app", itemId: id }];
  }
  updatePinnedItems(items: FavPinnedItem[]) { this.pinnedItems = items; }
  previewPinnedItems(items: FavPinnedItem[]) { pinnedItems = normalizeLayout(items); }
  createEmptyFolder(name = ""): FavFolderItem {
    const folder: FavFolderItem = { type: "folder", itemId: `folder:${crypto.randomUUID()}`, name, itemIds: [] };
    this.pinnedItems = [...pinnedItems, folder];
    this.page = Math.floor((pinnedItems.length - 1) / this.pageSize);
    this.view = StartView.Favorites;
    this.openFolderId = folder.itemId;
    return folder;
  }
  createFolder(first: string, second: string, targetIdx?: number): FavFolderItem {
    const folder: FavFolderItem = { type: "folder", itemId: `folder:${crypto.randomUUID()}`, name: "", itemIds: [...new Set([second, first])] };
    const remaining = removeApp(removeApp(pinnedItems, first), second);
    remaining.splice(Math.max(0, Math.min(targetIdx ?? remaining.length, remaining.length)), 0, folder);
    this.pinnedItems = remaining;
    return folder;
  }
  addItemToFolder(folderId: string, itemId: string) { this.pinnedItems = moveIntoFolder(pinnedItems, itemId, folderId); }
  moveItemToRoot(itemId: string) {
    this.pinnedItems = [...removeApp(pinnedItems, itemId), { type: "app", itemId }];
  }
  updateFolder(folderId: string, updates: Partial<Pick<FavFolderItem, "name" | "itemIds">>) {
    this.pinnedItems = pinnedItems.map(i => i.type === "folder" && i.itemId === folderId ? { ...i, ...updates } : i);
  }
  mergeFolders(sourceId: string, targetId: string) {
    if (sourceId === targetId) return;
    const source = this.folders.find(i => i.itemId === sourceId);
    if (!source || !this.folders.some(i => i.itemId === targetId)) return;
    this.pinnedItems = pinnedItems.filter(i => i.itemId !== sourceId).map(i => i.type === "folder" && i.itemId === targetId
      ? { ...i, itemIds: [...new Set([...i.itemIds, ...source.itemIds])] } : i);
  }
  disbandFolder(folderId: string) { this.pinnedItems = disbandFolder(pinnedItems, folderId); }
  moveRootItem(itemId: string, offset: number) { this.pinnedItems = moveRootItem(pinnedItems, itemId, offset); }

  getMenuItem(id: string): StartMenuItem | undefined {
    const item = startMenuItems.value.find(i => getItemId(i).toLowerCase() === id.toLowerCase())
      || foldersAsStartMenuItems.value.find(i => getItemId(i).toLowerCase() === id.toLowerCase());
    if (item) return item;
    if (id.includes("\\") || id.includes("/")) return {
      path: id, umid: null, display_name: (id.split(/[\\/]/g).pop() || id).replace(/\.lnk$/i, ""),
      target: null, toast_activator: null, start_menu_folder: [],
    };
    return undefined;
  }
}
export const globalState = new State();
