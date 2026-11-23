import { lazyRune } from "libs/ui/svelte/utils";
import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";

export const settings = lazyRune(() => invoke(MacUiCommand.StateGetSettings, { path: null }));
subscribe(MacUiEvent.StateSettingsChanged, settings.setByPayload);

export const focused = lazyRune(() => invoke(MacUiCommand.GetFocusedApp));
subscribe(MacUiEvent.GlobalFocusChanged, focused.setByPayload);

export const interactables = lazyRune(() => invoke(MacUiCommand.GetUserAppWindows));
subscribe(MacUiEvent.UserAppWindowsChanged, interactables.setByPayload);

export const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

export const virtualDesktops = lazyRune(() => invoke(MacUiCommand.StateGetVirtualDesktops));
subscribe(MacUiEvent.VirtualDesktopsChanged, virtualDesktops.setByPayload);

export const wallpapers = lazyRune(() => invoke(MacUiCommand.StateGetWallpapers));
subscribe(MacUiEvent.StateWallpapersChanged, wallpapers.setByPayload);

export const performanceMode = lazyRune(() => invoke(MacUiCommand.StateGetPerformanceMode));
subscribe(MacUiEvent.StatePerformanceModeChanged, performanceMode.setByPayload);

export const players = lazyRune(() => invoke(MacUiCommand.GetMediaSessions));
subscribe(MacUiEvent.MediaSessions, players.setByPayload);

await Promise.all([
  settings.init(),
  focused.init(),
  interactables.init(),
  monitors.init(),
  virtualDesktops.init(),
  wallpapers.init(),
  performanceMode.init(),
  players.init(),
]);
