import { invoke, MacUiCommand, MacUiEvent, type UnSubscriber } from "../../handlers/mod.ts";
import { List } from "../../utils/List.ts";
import type { BluetoothDevice } from "@macui/types";
import { newFromInvoke, newOnEvent } from "../../utils/State.ts";

export class BluetoothDevices extends List<BluetoothDevice> {
  static getAsync(): Promise<BluetoothDevices> {
    return newFromInvoke(this, MacUiCommand.GetBluetoothDevices);
  }

  static onChange(cb: (payload: BluetoothDevices) => void): Promise<UnSubscriber> {
    return newOnEvent(cb, this, MacUiEvent.BluetoothDevicesChanged);
  }

  static async discover(): Promise<void> {
    return await invoke(MacUiCommand.StartBluetoothScanning);
  }

  static async stopDiscovery(): Promise<void> {
    return await invoke(MacUiCommand.StopBluetoothScanning);
  }

  static default(): BluetoothDevices {
    return new this([]);
  }
}
