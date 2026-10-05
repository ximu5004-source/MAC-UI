<script lang="ts">
  import { globalState } from "./state.svelte";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { RadioDeviceKind } from "@seelen-ui/lib/types";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { t } from "./i18n";
  import BluetoothDevice from "./components/BluetoothDevice.svelte";
  import { Widget } from "@seelen-ui/lib";
  import { dedupeBluetoothDevices } from "libs/ui/svelte/utils/bluetoothIcons.ts";

  const uniqueDevices = $derived(dedupeBluetoothDevices(globalState.devices));

  const connectedDevices = $derived(uniqueDevices.filter((d) => d.paired && d.connected));
  const pairedDevices = $derived(uniqueDevices.filter((d) => d.paired && !d.connected));
  const availableDevices = $derived(uniqueDevices.filter((d) => !d.paired));

  const bluetoothRadio = $derived(globalState.bluetoothRadio);

  function openBluetoothSettings() {
    invoke(SeelenCommand.OpenFile, { path: "ms-settings:bluetooth" });
  }

  async function toggleBluetoothRadio() {
    if (bluetoothRadio) {
      await invoke(SeelenCommand.SetRadioState, {
        kind: RadioDeviceKind.Bluetooth,
        enabled: !bluetoothRadio.isEnabled,
      });
    }
  }

  $effect(() => {
    Widget.getCurrent().ready();
  });

  let closing = false;
  async function handleEscape(event: KeyboardEvent) {
    if (event.key !== "Escape" || event.repeat || closing) return;
    event.preventDefault();
    closing = true;
    try { await Widget.getCurrent().hide(); }
    catch (error) { console.error("Failed to hide Bluetooth popup:", error); }
    finally { closing = false; }
  }
</script>

<svelte:window onkeydown={handleEscape} />

<div class="slu-std-popover mac-panel mac-frosted-surface bluetooth-popup">
  {#if !bluetoothRadio}
    <div class="bluetooth-no-adapter">
      {$t("no_adapter")}
    </div>
  {:else}
    <div class="bluetooth-radio-control">
      <div class="bluetooth-radio-label">
        <Icon iconName="IoBluetooth" />
        <span>Bluetooth</span>
      </div>
      <label class="bluetooth-radio-switch">
        <input
          type="checkbox"
          data-skin="switch"
          aria-label="Bluetooth"
          checked={bluetoothRadio.isEnabled}
          onchange={toggleBluetoothRadio}
        />
      </label>
    </div>
  {/if}

  {#if bluetoothRadio?.isEnabled}
    <div class="bluetooth-body">
      {#if connectedDevices.length > 0}
        <div class="bt-list">
          <div class="bt-list-title">{$t("connected")}</div>
          <div class="bt-list-devices">
            {#each connectedDevices as device (device.id)}
              <BluetoothDevice {device} />
            {/each}
          </div>
        </div>
      {/if}

      {#if pairedDevices.length > 0}
        <div class="bt-list">
          <div class="bt-list-title">{$t("paired")}</div>
          <div class="bt-list-devices">
            {#each pairedDevices as device (device.id)}
              <BluetoothDevice {device} />
            {/each}
          </div>
        </div>
      {/if}

      <div class="bt-list">
        <div class="bt-list-title">
          <span>{$t("available")}</span>
          {#if globalState.isScanning}
            <div class="bluetooth-scanning">
              <Icon iconName="TbRefresh" />
            </div>
          {/if}
        </div>
        <div class="bt-list-devices">
          {#if availableDevices.length > 0}
            {#each availableDevices as device (device.id)}
              <BluetoothDevice {device} />
            {/each}
          {:else}
            <div class="bluetooth-empty">{$t("not_found")}</div>
          {/if}
        </div>
      </div>
    </div>

    <div class="bluetooth-footer">
      <button data-skin="transparent" onclick={openBluetoothSettings}>
        {$t("more")}
      </button>
    </div>
  {/if}
</div>

<style>
  :global(#root) .slu-std-popover.mac-panel.bluetooth-popup {
    border-radius: 28px;
    overflow: hidden;
  }
  .bluetooth-radio-control { flex-shrink: 0; }
  .bluetooth-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--mac-track) transparent;
  }
  .bt-list { flex: 0 0 auto; }
  .bt-list-title { flex-shrink: 0; }
  :global(#root) .bluetooth-popup .bt-list-devices {
    max-height: none;
    flex-shrink: 0;
    overflow: visible;
  }
  :global(#root) .bluetooth-popup :global(.bt-device) { flex-shrink: 0; }
</style>
