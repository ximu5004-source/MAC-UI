import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";

export const widget = Widget.getCurrent();

export const settings = lazyRune(() => Settings.getAsync());
await Settings.onChange((s) => (settings.value = s));

export const windows = lazyRune(async () =>
  (await invoke(MacUiCommand.GetUserAppWindows)).toSorted(
    (a, b) => b.lastForegroundAt - a.lastForegroundAt,
  )
);
subscribe(MacUiEvent.UserAppWindowsChanged, ({ payload }) => {
  windows.value = payload.toSorted((a, b) => b.lastForegroundAt - a.lastForegroundAt);
});

export const previews = lazyRune(() => invoke(MacUiCommand.GetUserAppWindowsPreviews));
subscribe(MacUiEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

export const focusedWinId = lazyRune(async () => (await invoke(MacUiCommand.GetFocusedApp)).hwnd);
subscribe(MacUiEvent.GlobalFocusChanged, (e) => {
  focusedWinId.value = e.payload.hwnd;
});

export const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

await Promise.all([
  settings.init(),
  windows.init(),
  previews.init(),
  focusedWinId.init(),
  monitors.init(),
]);
