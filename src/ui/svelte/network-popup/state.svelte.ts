import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import { type Hotspot, RadioDeviceKind, type WlanBssEntry } from "@macui/lib/types";
import { locale } from "./i18n/index.ts";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";

let widget = Widget.getCurrent();

const settings = lazyRune(() => Settings.getAsync());
Settings.onChange((s) => (settings.value = s));
await settings.init();

const isDevtoolsEnabled = $derived(settings.value.devTools);

$effect.root(() => {
  $effect(() => {
    locale.set(settings.value.language);
  });
});

let wlanBssEntries = $state<WlanBssEntry[]>([]);
subscribe(MacUiEvent.NetworkWlanScanned, ({ payload }) => {
  wlanBssEntries = payload;
});

let radios = lazyRune(() => invoke(MacUiCommand.GetRadios));
subscribe(MacUiEvent.RadiosChanged, radios.setByPayload);
await radios.init();

let hotspot = lazyRune(() => invoke(MacUiCommand.GetNetworkHotspot));
subscribe(MacUiEvent.NetworkHotspotChanged, hotspot.setByPayload);
await hotspot.init();

let isScanning = $state(false);
let selectedSsid = $state<string | null>(null);
let scanInterval: ReturnType<typeof setInterval> | null = null;
let currentView = $state<"main" | "hotspot">("main");

widget.window.onFocusChanged((e) => {
  if (e.payload) {
    isScanning = true;
  } else {
    isScanning = false;
    selectedSsid = null;
    currentView = "main";
  }
});

$effect.root(() => {
  $effect(() => {
    const wifiEnabled = radios.value.some(
      (radio) => radio.kind === RadioDeviceKind.WiFi && radio.isEnabled,
    );

    if (isScanning && wifiEnabled) {
      invoke(MacUiCommand.WlanScan);
      scanInterval = setInterval(() => invoke(MacUiCommand.WlanScan), 2000);
      return;
    }

    if (scanInterval !== null) {
      clearInterval(scanInterval);
      scanInterval = null;
    }
  });
});

class State {
  get wifiRadio() {
    return radios.value.find((radio) => radio.kind === RadioDeviceKind.WiFi);
  }

  get wlanBssEntries() {
    return wlanBssEntries;
  }

  get hotspot(): Hotspot | null {
    return hotspot.value;
  }

  get view() {
    return currentView;
  }

  set view(value: "main" | "hotspot") {
    currentView = value;
  }

  get isScanning() {
    return isScanning;
  }

  set isScanning(value: boolean) {
    isScanning = value;
  }

  get selectedSsid() {
    return selectedSsid;
  }

  set selectedSsid(value: string | null) {
    selectedSsid = value;
  }

  get isDevtoolsEnabled() {
    return isDevtoolsEnabled;
  }
}

export const globalState = new State();
