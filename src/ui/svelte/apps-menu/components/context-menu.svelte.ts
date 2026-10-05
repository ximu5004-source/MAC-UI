import { invoke, SeelenCommand, SeelenEvent, Widget } from "@seelen-ui/lib";
import type { ContextMenu, ContextMenuCallbackPayload, StartMenuItem, WidgetId } from "@seelen-ui/lib/types";
import { type FavFolderItem, globalState } from "../state/mod.svelte";
import { emit } from "@tauri-apps/api/event";
import { iconPackManager } from "libs/ui/svelte/components/Icon/common.svelte";

export const CONTEXT_MENU_ID = crypto.randomUUID();
export const CONTEXT_MENU_CALLBACK_EVENT = "apps_menu::context_menu_action";

export const FOLDER_CONTEXT_MENU_ID = crypto.randomUUID();
const FOLDER_CONTEXT_MENU_CALLBACK_EVENT = "apps_menu::folder_context_menu_action";
const MOVE_FOLDER_MENU_ID = crypto.randomUUID();

Widget.self.webview.listen<ContextMenuCallbackPayload>(
  CONTEXT_MENU_CALLBACK_EVENT,
  ({ payload }) => {
    const { key, meta } = payload;
    const item = (meta as any).item as StartMenuItem;

    if (key === "pin") {
      globalState.togglePin(item);
    } else if (key.startsWith("move-folder:")) {
      globalState.addItemToFolder(key.slice("move-folder:".length), globalState.getItemId(item));
    } else if (key === "move-root") {
      globalState.moveItemToRoot(globalState.getItemId(item));
    } else if (key === "new-folder") {
      const folder = globalState.createEmptyFolder();
      globalState.addItemToFolder(folder.itemId, globalState.getItemId(item));
    } else if (key === "previous-page" || key === "next-page") {
      globalState.moveRootItem(globalState.getItemId(item), globalState.pageSize * (key === "next-page" ? 1 : -1));
    } else if (key === "open_file_location") {
      Widget.self.hide();
      invoke(SeelenCommand.SelectFileOnExplorer, { path: item.path });
    } else if (key === "pin_to_dock") {
      emit(SeelenEvent.WegAddItem, {
        id: crypto.randomUUID(),
        displayName: item.display_name,
        umid: item.umid,
        path: item.path,
        pinned: true,
        preventPinning: false,
        relaunch: null,
      });
    } else if (key === "run_as_admin") {
      Widget.self.hide();
      const program = item.umid ? `shell:AppsFolder\\${item.umid}` : item.path;
      invoke(SeelenCommand.Run, { program, args: null, workingDir: null, elevated: true });
    } else if (key === "edit_icon") {
      const entry = iconPackManager.value.getIconEntry({
        path: item.path,
        umid: item.umid ?? null,
      });
      invoke(SeelenCommand.TriggerWidget, {
        payload: {
          id: "@seelen/icon-editor" as WidgetId,
          customArgs: { entry },
        },
      });
    }
  },
);

Widget.self.webview.listen<ContextMenuCallbackPayload>(
  FOLDER_CONTEXT_MENU_CALLBACK_EVENT,
  ({ payload }) => {
    const { key, meta } = payload;
    const folderId = (meta as any).folderId as string;

    if (key === "disband") {
      globalState.disbandFolder(folderId);
    } else if (key === "rename") {
      globalState.openFolderId = folderId;
    } else if (key === "previous-page" || key === "next-page") {
      globalState.moveRootItem(folderId, globalState.pageSize * (key === "next-page" ? 1 : -1));
    }
  },
);

export function getFolderContextMenu(
  folder: FavFolderItem,
  t: (key: string) => string,
): ContextMenu {
  return {
    identifier: FOLDER_CONTEXT_MENU_ID,
    meta: { folderId: folder.itemId },
    items: [
      { type: "Item", key: "rename", label: t("launchpad.rename_folder"), icon: "RiEditBoxLine", callbackEvent: FOLDER_CONTEXT_MENU_CALLBACK_EVENT },
      { type: "Item", key: "previous-page", label: t("launchpad.move_previous_page"), icon: "FiChevronLeft", callbackEvent: FOLDER_CONTEXT_MENU_CALLBACK_EVENT },
      { type: "Item", key: "next-page", label: t("launchpad.move_next_page"), icon: "FiChevronRight", callbackEvent: FOLDER_CONTEXT_MENU_CALLBACK_EVENT },
      {
        type: "Item",
        key: "disband",
        label: t("disband"),
        icon: "GiExpand",
        callbackEvent: FOLDER_CONTEXT_MENU_CALLBACK_EVENT,
      },
    ],
  };
}

export function getItemContextMenu(item: StartMenuItem, t: (key: string) => string): ContextMenu {
  const isPinned = globalState.isPinned(item);
  const umid = item.umid;
  const path = item.path.toLowerCase();

  const items: ContextMenu["items"] = [
    {
      type: "Item",
      key: "pin",
      label: isPinned ? t("unpin") : t("pin"),
      icon: isPinned ? "TbPinnedOff" : "TbPin",
      callbackEvent: CONTEXT_MENU_CALLBACK_EVENT,
    },
  ];

  const itemId = globalState.getItemId(item);
  const parent = globalState.folders.find(folder => folder.itemIds.includes(itemId));
  if (parent) items.push({ type: "Item", key: "move-root", label: t("launchpad.move_out"), icon: "FiArrowUpLeft", callbackEvent: CONTEXT_MENU_CALLBACK_EVENT });
  const destinations = globalState.folders.filter(folder => folder.itemId !== parent?.itemId);
  items.push({
    type: "Submenu", identifier: MOVE_FOLDER_MENU_ID, label: t("launchpad.move_to_folder"), icon: "FiFolder",
    items: [
      { type: "Item", key: "new-folder", label: t("launchpad.new_folder"), icon: "FiFolderPlus", callbackEvent: CONTEXT_MENU_CALLBACK_EVENT },
      ...destinations.map(folder => ({ type: "Item" as const, key: `move-folder:${folder.itemId}`, label: folder.name || t("folder"), callbackEvent: CONTEXT_MENU_CALLBACK_EVENT })),
    ],
  });
  if (globalState.pinnedItems.some(entry => entry.type === "app" && entry.itemId === itemId)) {
    items.push(
      { type: "Item", key: "previous-page", label: t("launchpad.move_previous_page"), icon: "FiChevronLeft", callbackEvent: CONTEXT_MENU_CALLBACK_EVENT },
      { type: "Item", key: "next-page", label: t("launchpad.move_next_page"), icon: "FiChevronRight", callbackEvent: CONTEXT_MENU_CALLBACK_EVENT },
    );
  }

  if (path) {
    items.push({
      type: "Item" as const,
      key: "open_file_location",
      label: t("open_file_location"),
      icon: "MdOutlineMyLocation",
      callbackEvent: CONTEXT_MENU_CALLBACK_EVENT,
    });
  }

  if (umid || path.endsWith(".exe") || path.endsWith(".lnk")) {
    items.push({
      type: "Item" as const,
      key: "run_as_admin",
      label: t("run_as_admin"),
      icon: "MdOutlineAdminPanelSettings",
      callbackEvent: CONTEXT_MENU_CALLBACK_EVENT,
    });
  }

  items.push({
    type: "Item",
    key: "edit_icon",
    label: t("edit_icon"),
    icon: "RiEditBoxLine",
    callbackEvent: CONTEXT_MENU_CALLBACK_EVENT,
  });

  return {
    identifier: CONTEXT_MENU_ID,
    meta: { item },
    items,
  };
}
