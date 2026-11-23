import { invoke, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import { RadioDeviceKind } from "@macui/lib/types";
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

let devices = lazyRune(() => invoke(MacUiCommand.GetBluetoothDevices));
subscribe(MacUiEvent.BluetoothDevicesChanged, devices.setByPayload);

let radios = lazyRune(() => invoke(MacUiCommand.GetRadios));
subscribe(MacUiEvent.RadiosChanged, radios.setByPayload);

await Promise.all([devices.init(), radios.init()]);

export enum BluetoothOperation {
  Connecting = "connecting",
  Disconnecting = "disconnecting",
  Pairing = "pairing",
  Unpairing = "unpairing",
}

let isScanning = $state(false);
let selectedDeviceId = $state<string | null>(null);
let loadingDeviceId = $state<string | null>(null);
let loadingOperation = $state<BluetoothOperation | null>(null);

widget.window.onFocusChanged(async (e) => {
  if (e.payload) {
    await invoke(MacUiCommand.StartBluetoothScanning);
    isScanning = true;
  } else {
    await invoke(MacUiCommand.StopBluetoothScanning);
    isScanning = false;
    selectedDeviceId = null;
  }
});

class State {
  get bluetoothRadio() {
    return radios.value.find((radio) => radio.kind === RadioDeviceKind.Bluetooth);
  }

  get devices() {
    return devices.value;
  }

  get isScanning() {
    return isScanning;
  }
  set isScanning(value: boolean) {
    isScanning = value;
  }

  get selectedDeviceId() {
    return selectedDeviceId;
  }
  set selectedDeviceId(value: string | null) {
    selectedDeviceId = value;
  }

  get loadingDeviceId() {
    return loadingDeviceId;
  }
  set loadingDeviceId(value: string | null) {
    loadingDeviceId = value;
  }

  get loadingOperation() {
    return loadingOperation;
  }
  set loadingOperation(value: BluetoothOperation | null) {
    loadingOperation = value;
  }

  get isDevtoolsEnabled() {
    return isDevtoolsEnabled;
  }
}

export const globalState = new State();
