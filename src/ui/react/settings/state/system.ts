import { lazySignal } from "libs/ui/react/utils/LazySignal";
import { invoke, MacUiCommand, MacUiEvent, subscribe, UIColors } from "@macui/lib";
import { effect, signal } from "@preact/signals";

export const uiColors = lazySignal(() => invoke(MacUiCommand.SystemGetColors));
subscribe(MacUiEvent.ColorsChanged, uiColors.setByPayload);
await uiColors.init();

effect(() => {
  new UIColors(uiColors.value).setAsCssVariables();
});

export const monitors = lazySignal(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);
await monitors.init();

export const autostart = signal(await invoke(MacUiCommand.GetAutoStartStatus));
export async function setAutoStart(enabled: boolean) {
  await invoke(MacUiCommand.SetAutoStart, { enabled });
  autostart.value = enabled;
}
