import { invoke, PluginList, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import type { UserAppWindowColors } from "@macui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";

export const currentMonitorId = Widget.getCurrent().decoded.monitorId!;

export const virtualDesktop = lazyRune(async () => {
  const initialDesktops = await invoke(MacUiCommand.StateGetVirtualDesktops);
  return initialDesktops.monitors[currentMonitorId];
});
subscribe(MacUiEvent.VirtualDesktopsChanged, (e) => {
  virtualDesktop.value = e.payload.monitors[currentMonitorId];
});

export const virtualDesktops = lazyRune(() => invoke(MacUiCommand.StateGetVirtualDesktops));
subscribe(MacUiEvent.VirtualDesktopsChanged, virtualDesktops.setByPayload);

export const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

export const mousePos = lazyRune(() => invoke(MacUiCommand.GetMousePosition));
subscribe(MacUiEvent.GlobalMouseMove, mousePos.setByPayload);

export const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));

export const interactables = lazyRune(() => invoke(MacUiCommand.GetUserAppWindows));
subscribe(MacUiEvent.UserAppWindowsChanged, interactables.setByPayload);

export const windowsColors = lazyRune<Record<number, UserAppWindowColors>>(
  () => invoke(MacUiCommand.GetUserAppWindowsColors),
);
subscribe(MacUiEvent.UserAppWindowsColorsChanged, windowsColors.setByPayload);

export const focused = lazyRune(() => invoke(MacUiCommand.GetFocusedApp));
subscribe(MacUiEvent.GlobalFocusChanged, focused.setByPayload);

export const widgetStatuses = lazyRune(() => invoke(MacUiCommand.DebugGetWidgetsStatuses));
subscribe(MacUiEvent.WidgetDebugInfoChanged, widgetStatuses.setByPayload);

export const toolbarItems = lazyRune(() => invoke(MacUiCommand.StateGetToolbarItems));

export const plugins = lazyRune(async () => (await PluginList.getAsync()).forCurrentWidget());
PluginList.onChange((list) => {
  plugins.value = list.forCurrentWidget();
});

await Promise.all([
  virtualDesktop.init(),
  virtualDesktops.init(),
  monitors.init(),
  mousePos.init(),
  settings.init(),
  interactables.init(),
  windowsColors.init(),
  focused.init(),
  widgetStatuses.init(),
  toolbarItems.init(),
  plugins.init(),
]);
