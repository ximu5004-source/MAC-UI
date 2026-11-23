import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe } from "@macui/lib";
import type { Hotspot, MediaDevice, PhysicalMonitor, RadioDevice } from "@macui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import { locale } from "./i18n/index.ts";

const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));
await settings.init();

$effect.root(() => {
  $effect(() => {
    locale.set(settings.value.language);
  });
});

// Initialize lazy signals
const brightness = lazyRune(() => invoke(MacUiCommand.GetAllMonitorsBrightness));
subscribe(MacUiEvent.SystemMonitorsBrightnessChanged, brightness.setByPayload);

const mediaDevices = lazyRune(async () => {
  const [inputs, outputs] = await invoke(MacUiCommand.GetMediaDevices);
  return { inputs, outputs };
});
subscribe(MacUiEvent.MediaDevices, ({ payload: [inputs, outputs] }) => {
  mediaDevices.value = { inputs, outputs };
});

const radios = lazyRune(() => invoke(MacUiCommand.GetRadios));
subscribe(MacUiEvent.RadiosChanged, radios.setByPayload);

const hotspot = lazyRune(() => invoke(MacUiCommand.GetNetworkHotspot));
subscribe(MacUiEvent.NetworkHotspotChanged, hotspot.setByPayload);

const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

const darkMode = lazyRune(() => invoke(MacUiCommand.SystemGetDarkMode));
subscribe(MacUiEvent.DarkModeChanged, darkMode.setByPayload);

const nightLightEnabled = lazyRune(() => invoke(MacUiCommand.SystemGetNightLightEnabled));

await Promise.all([
  brightness.init(),
  mediaDevices.init(),
  radios.init(),
  hotspot.init(),
  monitors.init(),
  darkMode.init(),
  nightLightEnabled.init(),
]);

class State {
  get brightness() {
    return brightness.value;
  }

  get mediaInputs(): MediaDevice[] {
    return mediaDevices.value.inputs;
  }
  get mediaOutputs(): MediaDevice[] {
    return mediaDevices.value.outputs;
  }
  get radios(): RadioDevice[] {
    return radios.value;
  }
  get hotspot(): Hotspot | null {
    return hotspot.value;
  }
  get monitors(): PhysicalMonitor[] {
    return monitors.value;
  }
  get darkMode(): boolean {
    return darkMode.value;
  }
  get nightLightEnabled(): boolean {
    return nightLightEnabled.value;
  }
  set nightLightEnabled(value: boolean) {
    nightLightEnabled.value = value;
  }
}
export const state = new State();
