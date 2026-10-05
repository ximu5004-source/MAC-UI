<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { HotspotState, RadioDeviceKind, type RadioDevice, type WidgetId } from "@seelen-ui/lib/types";
  import { state } from "../state.svelte";
  import type { IconName } from "libs/ui/icons";
  import { t } from "../i18n";
  import QuickToggle from "./QuickToggle.svelte";

  const hdrMonitors = $derived(state.monitors.filter((m) => m.hdr !== null && m.hdr !== undefined));
  const hdrEnabled = $derived(hdrMonitors.length > 0 && hdrMonitors.every((m) => m.hdr));

  function getRadioIcon(kind: RadioDeviceKind): IconName {
    switch (kind) {
      case RadioDeviceKind.WiFi: return "IoWifiSharp";
      case RadioDeviceKind.Bluetooth: return "IoBluetooth";
      case RadioDeviceKind.MobileBroadband: return "IoPhonePortraitSharp";
      case RadioDeviceKind.FM: return "IoRadio";
      case RadioDeviceKind.Other: return "IoRadioButtonOnSharp";
    }
  }

  function getRadioLabel(kind: RadioDeviceKind): string {
    switch (kind) {
      case RadioDeviceKind.WiFi: return "Wi-Fi";
      case RadioDeviceKind.Bluetooth: return "Bluetooth";
      case RadioDeviceKind.MobileBroadband: return $t("mobile_broadband");
      case RadioDeviceKind.FM: return $t("fm_radio");
      default: return $t("unknown");
    }
  }

  function openDetails(id: string) {
    return invoke(SeelenCommand.TriggerWidget, { payload: { id: id as WidgetId } });
  }

  function radioDetails(kind: RadioDeviceKind) {
    if (kind === RadioDeviceKind.WiFi) return () => openDetails("@seelen/network-popup");
    if (kind === RadioDeviceKind.Bluetooth) return () => openDetails("@seelen/bluetooth-popup");
    return undefined;
  }

  async function toggleRadio(radio: RadioDevice) {
    await invoke(SeelenCommand.SetRadioState, { kind: radio.kind, enabled: !radio.isEnabled });
  }

  async function toggleHotspot() {
    if (!state.hotspot) return;
    await invoke(SeelenCommand.SetNetworkHotspotState, {
      enabled: state.hotspot.state !== HotspotState.on,
    });
  }

  async function toggleHdr() {
    const newState = !hdrEnabled;
    await Promise.all(hdrMonitors.map((monitor) =>
      invoke(SeelenCommand.SetMonitorHdr, { id: monitor.id, state: newState })
    ));
  }

  async function toggleDarkMode() {
    await invoke(SeelenCommand.SystemSetDarkMode, { enabled: !state.darkMode });
  }

  async function toggleNightLight() {
    const enabled = !state.nightLightEnabled;
    await invoke(SeelenCommand.SystemSetNightLightEnabled, { enabled });
    state.nightLightEnabled = enabled;
  }
</script>

<div class="quick-toggle-grid">
  {#each state.radios as radio (radio.id)}
    <QuickToggle
      icon={getRadioIcon(radio.kind)} label={getRadioLabel(radio.kind)}
      status={$t(radio.isEnabled ? "enabled" : "disabled")} enabled={radio.isEnabled}
      onToggle={() => toggleRadio(radio)} onDetails={radioDetails(radio.kind)}
    />
  {/each}
  {#if state.hotspot}
    <QuickToggle icon="MdWifiTethering" label={$t("hotspot")}
      status={$t(state.hotspot.state === HotspotState.on ? "enabled" : "disabled")}
      enabled={state.hotspot.state === HotspotState.on}
      disabled={state.hotspot.state === HotspotState.inTransition} onToggle={toggleHotspot} />
  {/if}
  {#if hdrMonitors.length > 0}
    <QuickToggle icon="TbHdr" label={$t("hdr")}
      status={$t(hdrEnabled ? "enabled" : "disabled")} enabled={hdrEnabled} onToggle={toggleHdr} />
  {/if}
  <QuickToggle icon={state.darkMode ? "IoMoon" : "IoSunny"}
    label={state.darkMode ? $t("dark_mode") : $t("light_mode")}
    status={$t("appearance")} enabled={state.darkMode} onToggle={toggleDarkMode} />
  <QuickToggle icon="IoEye" label={$t("eye_care")}
    status={$t(state.nightLightEnabled ? "enabled" : "disabled")}
    enabled={state.nightLightEnabled} onToggle={toggleNightLight} />
</div>
