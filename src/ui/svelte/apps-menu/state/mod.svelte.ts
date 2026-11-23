import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe } from "@macui/lib";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { StartMenuItem } from "@macui/lib/types";
import { lazyRune, persistentRune } from "libs/ui/svelte/utils";
import { StartDisplayMode, StartView } from "../constants";
import { locale } from "../i18n/index.ts";
import { clearFavoritesSafely } from "../favoritesClear";
import { FAVORITES_FILE, loadFavoritesStorage } from "../favoritesStorage";
import {
  disbandFolder, getItemId, moveIntoFolder, moveRootItem,
  normalizeLayout, removeApp, startMenuCatalog,
  type FavFolderItem, type FavPinnedItem, type LaunchpadLayout,
} from "../launchpad";
export type { FavAppItem, FavFolderItem, FavPinnedItem } from "../launchpad";

const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));
const user = lazyRune(() => invoke(MacUiCommand.GetUser));
subscribe(MacUiEvent.UserChanged, user.setByPayload);
const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);
const startMenuItems = lazyRune(() => invoke(MacUiCommand.GetStartMenuItems));
subscribe(MacUiEvent.StartMenuItemsChanged, startMenuItems.setByPayload);
await Promise.all([settings.init(), user.init(), monitors.init(), startMenuItems.init()]);

$effect.root(() => { $effect(() => { locale.set(settings.value.language); }); });

// Preserve v1 intact for rollback; v2 never auto-imports the application catalog.
const STORAGE_FILE = FAVORITES_FILE;
let pinnedItems = $state<FavPinnedItem[]>([]);
let saveState = $state<"saved" | "saving" | "error">("saved");
let refreshing = $state(false);
let refreshFailed = $state(false);
let choosingFavorites = $state(false);
let retainedLegacyLayout = $state(false);
let clearingFavorites = $state(false);
let clearFavoritesError = $state(false);
let clearUndo = $state<FavPinnedItem[] | null>(null);
let layoutLoaded = $state(false);
let layoutLoading = $state(false);
let layoutLoadError = $state(false);
let writeQueue: Promise<void> = Promise.resolve();
let revision = 0;

function saveItems(items: FavPinnedItem[]): Promise<void> {
  if (!layoutLoaded) return Promise.reject(new Error("Favorites are not loaded; refusing to overwrite saved layout"));
  clearUndo = null;
  pinnedItems = normalizeLayout(items);
  const content = JSON.stringify({ version: 2, items: pinnedItems } satisfies LaunchpadLayout);
  const currentRevision = ++revision;
  saveState = "saving";
  const save = writeQueue.catch(() => {}).then(() => invoke(MacUiCommand.WriteFile, { filename: STORAGE_FILE, content }));
  writeQueue = save;
  void save.then(() => {
    if (revision === currentRevision) saveState = "saved";
  }, (error) => {
    console.error("Launchpad layout save failed:", error);
    if (revision === currentRevision) saveState = "error";
  });
  return save;
}

async function loadFavorites() {
  if (layoutLoading || layoutLoaded) return;
  layoutLoading = true;
  layoutLoadError = false;
  try {
    const loaded = await loadFavoritesStorage(
      filename => invoke(MacUiCommand.ReadFile, { filename }), startMenuCatalog(startMenuItems.value),
    );
    pinnedItems = loaded.items;
    retainedLegacyLayout = loaded.migrated;
    layoutLoaded = true;
    if (loaded.needsSave) await saveItems(loaded.items).catch(() => {});
  } catch (error) {
    layoutLoadError = true;
    console.error("Could not read favorites; existing layout is preserved:", error);
  } finally { layoutLoading = false; }
}
await loadFavorites();

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
      const path = await invoke(MacUiCommand.GetNativeShellWallpaper);
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
  get refreshing() { return refreshing; }
  get refreshFailed() { return refreshFailed; }
  get retainedLegacyLayout() { return retainedLegacyLayout; }
  get choosingFavorites() { return choosingFavorites; }
  set choosingFavorites(value: boolean) { choosingFavorites = value; }
  get clearingFavorites() { return clearingFavorites; }
  get clearFavoritesError() { return clearFavoritesError; }
  get canUndoClear() { return clearUndo !== null; }
  get layoutLoadError() { return layoutLoadError; }
  get layoutLoading() { return layoutLoading; }
  get canEditFavorites() { return layoutLoaded && !layoutLoading; }
  get displayMode() { return displayMode.value; }
  set displayMode(value: StartDisplayMode) { displayMode.value = value; }
  get folders() { return pinnedItems.filter((i): i is FavFolderItem => i.type === "folder"); }

  retrySave() {
    if (!layoutLoaded) { void loadFavorites(); return; }
    void saveItems(pinnedItems).catch(() => {});
  }

  chooseFavorites() {
    if (!layoutLoaded) return;
    choosingFavorites = true;
    this.view = StartView.All;
    this.page = 0;
  }

  async clearFavorites() {
    if (!layoutLoaded || clearingFavorites || !pinnedItems.length) return false;
    clearingFavorites = true;
    clearFavoritesError = false;
    let completed = false;
    const snapshot = normalizeLayout(pinnedItems);
    try {
      await clearFavoritesSafely(snapshot, {
        getRevision: () => revision,
        // Explicit user action only; retain a durable copy before any reset.
        backup: items => invoke(MacUiCommand.WriteFile, {
          filename: `launchpad-favorites-before-clear-${Date.now()}-${crypto.randomUUID()}.json`,
          content: JSON.stringify({ version: 2, items } satisfies LaunchpadLayout),
        }),
        clear: () => {
          const saved = saveItems([]);
          return { revision, saved };
        },
        restore: items => {
          ++revision;
          pinnedItems = normalizeLayout(items);
          clearUndo = normalizeLayout(items);
          saveState = "error";
        },
        complete: items => {
          clearUndo = items;
          retainedLegacyLayout = false;
          openFolderId = null;
          this.page = 0;
          completed = true;
        },
      });
    } catch (error) {
      clearFavoritesError = true;
      console.error("Could not clear favorites safely:", error);
    } finally { clearingFavorites = false; }
    return completed;
  }
  undoClearFavorites() {
    if (clearUndo) this.pinnedItems = clearUndo;
  }

  async refreshApplications() {
    if (refreshing) return;
    refreshing = true;
    refreshFailed = false;
    try {
      startMenuItems.value = await invoke(MacUiCommand.GetStartMenuItems);
    } catch (error) {
      refreshFailed = true;
      console.error("Start Menu refresh failed:", error);
    } finally { refreshing = false; }
  }

  isPinned(item: StartMenuItem) {
    const id = getItemId(item).toLowerCase();
    return pinnedItems.some(i => i.type === "app" ? i.itemId.toLowerCase() === id : i.itemIds.some(child => child.toLowerCase() === id));
  }
  togglePin(item: StartMenuItem) {
    if (!layoutLoaded || clearingFavorites) return;
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
    // Unavailable legacy IDs stay saved, but never fabricate a launchable app
    // from a Downloads/Desktop installer or an uninstalled shortcut.
    return this.allItems.find(i => getItemId(i).toLowerCase() === id.toLowerCase()
      || (i.path && i.path.toLowerCase() === id.toLowerCase()));
  }
}
export const globalState = new State();
