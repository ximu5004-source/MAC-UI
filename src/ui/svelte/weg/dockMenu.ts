import { invoke, MacUiCommand, Settings, Widget } from "@macui/lib";
import { dialog } from "@macui/lib/tauri";
import { WegWindowAnimation } from "@macui/lib/types";
import type { ContextMenu, ContextMenuCallbackPayload, ContextMenuItem, PluginId, WidgetId } from "@macui/lib/types";
import { getResourceText } from "libs/ui/react/utils/index.ts";
import { locale } from "./i18n/index.ts";
import { dockState, dockStateActions } from "./state/items.svelte.ts";
import { isHorizontalDock } from "./state/settings.svelte.ts";
import { plugins, settings } from "./state/getters.svelte.ts";

const MENU_ID = crypto.randomUUID();
const PLUGINS_SUBMENU_ID = crypto.randomUUID();
const ANIMATION_SUBMENU_ID = crypto.randomUUID();

const MENU_EVENT = "weg::bar_menu_click";
const PLUGINS_SUBMENU_EVENT = "weg::toggle_plugin";

let _t: (key: string) => string = (key) => key;
let animationSave = Promise.resolve();

function saveWindowAnimation(animation: WegWindowAnimation): Promise<void> {
  // Read the latest backend state for each choice, rather than writing the
  // possibly stale menu snapshot over unrelated settings. Serialize rapid clicks.
  const save = animationSave.then(async () => {
    const latest = await Settings.getAsync();
    latest.inner.byWidget["@macui/weg"] = {
      ...latest.inner.byWidget["@macui/weg"],
      windowAnimation: animation,
    };
    await latest.save();
    settings.value = latest;
  });
  animationSave = save.catch(() => {});
  return save;
}

async function handleBarMenuClick({ key, value }: ContextMenuCallbackPayload) {
  switch (key) {
    case "window-animation":
      if (value === WegWindowAnimation.Genie || value === WegWindowAnimation.Native) {
        try {
          await saveWindowAnimation(value);
        } catch (error) {
          console.error("Could not save Dock window animation", error);
          await dialog.message(_t("window_animation.save_failed"), {
            title: _t("window_animation.label"),
            kind: "error",
          });
        }
      }
      break;
    case "animation-settings":
      await invoke(MacUiCommand.TriggerWidget, {
        payload: {
          id: "@macui/settings" as WidgetId,
          customArgs: { route: "/widget?id=%40macui%2Fweg" },
        },
      });
      break;
    case "add-separator":
      dockStateActions.addSeparatorNear(value as { x: number; y: number });
      break;
    case "reorder":
      dockState.state = {
        ...dockState.state,
        isReorderDisabled: !dockState.isReorderDisabled,
      };
      break;
    case "task_manager":
      invoke(MacUiCommand.OpenFile, { path: "Taskmgr.exe" });
      break;
    case "settings":
      invoke(MacUiCommand.TriggerWidget, {
        payload: { id: "@macui/settings" as WidgetId },
      });
      break;
    case "add-item": {
      const files = await dialog.open({
        title: _t("taskbar_menu.add_file"),
        multiple: true,
        filters: [
          { name: "lnk", extensions: ["lnk"] },
          { name: "*", extensions: ["*"] },
        ],
      });
      for (const path of files || []) {
        await invoke(MacUiCommand.WegPinItem, { path });
      }
      break;
    }
  }
}

Widget.self.webview.listen<ContextMenuCallbackPayload>(MENU_EVENT, ({ payload }) => {
  handleBarMenuClick(payload).catch(console.error);
});

Widget.self.webview.listen<ContextMenuCallbackPayload>(PLUGINS_SUBMENU_EVENT, ({ payload }) => {
  const { key, checked } = payload;
  if (checked) {
    dockStateActions.addPlugin(key as PluginId);
  } else {
    dockStateActions.removePlugin(key as PluginId);
  }
});

export function getMacUiWegMenu(
  t: (key: string) => string,
  cursor: { x: number; y: number },
): ContextMenu {
  _t = t;
  const { isReorderDisabled } = dockState;
  const language = locale.value;
  const animation = settings.value.byWidget["@macui/weg"].windowAnimation ?? WegWindowAnimation.Genie;

  function isPluginAdded(id: PluginId): boolean {
    return dockState.items.some((item) => item.type === "Plugin" && item.plugin === id);
  }

  const pluginList = [
    ...plugins.value.map((plugin) => ({
      type: "Item",
      key: plugin.id,
      label: getResourceText(plugin.metadata.displayName, language),
      icon: plugin.icon,
      callbackEvent: PLUGINS_SUBMENU_EVENT,
      checked: isPluginAdded(plugin.id),
    })),
  ].toSorted((p1, p2) => p1.label.localeCompare(p2.label)) as ContextMenuItem[];

  return {
    identifier: MENU_ID,
    items: [
      {
        type: "Submenu",
        icon: "RiMagicLine",
        label: t("window_animation.label"),
        identifier: ANIMATION_SUBMENU_ID,
        items: [
          ...[WegWindowAnimation.Genie, WegWindowAnimation.Native].map((choice) => ({
            type: "Item" as const,
            key: "window-animation",
            value: choice,
            label: t(choice === WegWindowAnimation.Genie ? "window_animation.genie" : "window_animation.native"),
            icon: animation === choice ? "IoCheckmark" : undefined,
            // These are exclusive actions, not independent toggles. Closing the
            // menu avoids optimistic checkbox state disagreeing with a save.
            checked: null,
            callbackEvent: MENU_EVENT,
          })),
          { type: "Separator" },
          {
            type: "Item",
            key: "animation-settings",
            icon: "RiSettings4Line",
            label: t("window_animation.settings"),
            callbackEvent: MENU_EVENT,
          },
        ],
      },
      {
        type: "Submenu",
        icon: "CgExtensionAdd",
        label: t("taskbar_menu.modules"),
        identifier: PLUGINS_SUBMENU_ID,
        items: pluginList,
      },
      { type: "Separator" },
      {
        type: "Item",
        key: "add-item",
        icon: "RiFileAddLine",
        label: t("taskbar_menu.add_file"),
        callbackEvent: MENU_EVENT,
      },
      {
        type: "Item",
        key: "add-separator",
        icon: isHorizontalDock() ? "LuSeparatorVertical" : "LuSeparatorHorizontal",
        label: t("taskbar_menu.add_separator"),
        value: cursor,
        callbackEvent: MENU_EVENT,
      },
      { type: "Separator" },
      {
        type: "Item",
        key: "reorder",
        icon: isReorderDisabled ? "CgLockUnlock" : "CgLock",
        label: t(
          isReorderDisabled ? "context_menu.reorder_enable" : "context_menu.reorder_disable",
        ),
        callbackEvent: MENU_EVENT,
      },
      {
        type: "Item",
        key: "task_manager",
        icon: "PiChartLineFill",
        label: t("taskbar_menu.task_manager"),
        callbackEvent: MENU_EVENT,
      },
      {
        type: "Item",
        key: "settings",
        icon: "RiSettings4Fill",
        label: t("taskbar_menu.settings"),
        callbackEvent: MENU_EVENT,
      },
    ],
  };
}
