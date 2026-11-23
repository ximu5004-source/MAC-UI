import { invoke, MacUiCommand, Widget } from "@macui/lib";
import type { ContextMenu, ContextMenuCallbackPayload, ContextMenuItem } from "@macui/lib/types";
import type { SwItem } from "./types.ts";
import { dockStateActions, HARDCODED_SEPARATOR_LEFT, HARDCODED_SEPARATOR_RIGHT } from "./state/items.svelte.ts";

const identifier = crypto.randomUUID();
const onItemMenuClick = "weg::item_menu_click";

let pendingItem: SwItem | null = null;

Widget.self.webview.listen<ContextMenuCallbackPayload>(onItemMenuClick, ({ payload }) => {
  const { key } = payload;
  const item = pendingItem;
  if (!item) return;

  if (key === "remove") {
    dockStateActions.remove(item.id);
  } else if (key === "empty_bin") {
    invoke(MacUiCommand.TrashBinEmpty);
  }
});

export function getEmptyTrashBinEntry(t: (key: string) => string): ContextMenuItem {
  return {
    type: "Item",
    key: "empty_bin",
    icon: "FaRegTrashAlt",
    label: t("trash_bin.empty_bin"),
    callbackEvent: onItemMenuClick,
  };
}

export function getMenuForItem(t: (key: string) => string, item: SwItem): ContextMenu {
  pendingItem = item;

  if (item.type === "Media" || item.type === "Plugin") {
    const items: ContextMenuItem[] = [
      {
        type: "Item",
        key: "remove",
        icon: "CgExtensionRemove",
        label: t("context_menu.remove_module"),
        callbackEvent: onItemMenuClick,
      },
    ];

    return { identifier, items };
  }

  if (
    item.type === "Separator" &&
    item.id !== HARDCODED_SEPARATOR_LEFT.id &&
    item.id !== HARDCODED_SEPARATOR_RIGHT.id
  ) {
    const items: ContextMenuItem[] = [
      {
        type: "Item",
        key: "remove",
        icon: "IoRemove",
        label: t("context_menu.remove_separator"),
        callbackEvent: onItemMenuClick,
      },
    ];

    return { identifier, items };
  }

  return { identifier, items: [] };
}
