export const SeelenCommand = {
  SetVolumeLevel: "set_volume_level",
  MediaToggleMute: "media_toggle_mute",
  SetMonitorBrightness: "set_monitor_brightness",
  SetFrostedRegions: "set_frosted_regions",
};

export async function invoke(command: string, payload?: Record<string, unknown>) {
  if (command === SeelenCommand.SetFrostedRegions) {
    throw new Error("Native backdrop composition is unsupported in the browser fixture");
  }
  if ([SeelenCommand.SetVolumeLevel, SeelenCommand.MediaToggleMute, SeelenCommand.SetMonitorBrightness].includes(command)) {
    window.dispatchEvent(new CustomEvent("qa-native-command", { detail: { command, payload } }));
    return;
  }
  throw new Error(`Unsupported fixture command: ${command}`);
}
