import { invoke, PluginList, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import type { UserAppWindowColors } from "@macui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";

export const currentMonitorId = Widget.getCurrent().decoded.monitorId!;

export const virtualDesktops = lazyRune(() => invoke(MacUiCommand.StateGetVirtualDesktops));
subscribe(MacUiEvent.VirtualDesktopsChanged, virtualDesktops.setByPayload);

export const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

export const players = lazyRune(() => invoke(MacUiCommand.GetMediaSessions));
subscribe(MacUiEvent.MediaSessions, players.setByPayload);

export const notifications = lazyRune(() => invoke(MacUiCommand.GetNotifications).catch((error) => {
  console.warn("Dock notification badges unavailable:", error);
  return [];
}));
subscribe(MacUiEvent.Notifications, notifications.setByPayload);

export const mousePos = lazyRune(async () => {
  const [x, y] = await invoke(MacUiCommand.GetMousePosition);
  return { x, y };
});
subscribe(MacUiEvent.GlobalMouseMove, ({ payload: [x, y] }) => {
  mousePos.value = { x, y };
});

export const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));

export const selfWinId = lazyRune(() => invoke(MacUiCommand.GetSelfWindowId));

export const interactables = lazyRune(() => invoke(MacUiCommand.GetUserAppWindows));
subscribe(MacUiEvent.UserAppWindowsChanged, interactables.setByPayload);

export const previews = lazyRune(() => invoke(MacUiCommand.GetUserAppWindowsPreviews));
subscribe(MacUiEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

export const windowsColors = lazyRune<Record<number, UserAppWindowColors>>(
  () => invoke(MacUiCommand.GetUserAppWindowsColors),
);
subscribe(MacUiEvent.UserAppWindowsColorsChanged, windowsColors.setByPayload);

export const focused = lazyRune(() => invoke(MacUiCommand.GetFocusedApp));

export const widgetStatuses = lazyRune(() => invoke(MacUiCommand.DebugGetWidgetsStatuses));
subscribe(MacUiEvent.WidgetDebugInfoChanged, widgetStatuses.setByPayload);

export const wegItems = lazyRune(() => invoke(MacUiCommand.StateGetWegItems));

export const plugins = lazyRune(async () => (await PluginList.getAsync()).forCurrentWidget());
PluginList.onChange((list) => {
  plugins.value = list.forCurrentWidget();
});

await Promise.all([
  virtualDesktops.init(),
  monitors.init(),
  players.init(),
  notifications.init(),
  mousePos.init(),
  settings.init(),
  selfWinId.init(),
  interactables.init(),
  previews.init(),
  windowsColors.init(),
  focused.init(),
  widgetStatuses.init(),
  wegItems.init(),
  plugins.init(),
]);
