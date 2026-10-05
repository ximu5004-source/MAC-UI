<script lang="ts">
  import QRCode from "qrcode";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { HotspotState } from "@seelen-ui/lib/types";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { globalState } from "../state.svelte";
  import { t } from "../i18n";

  interface Props {
    onBack: () => void;
  }

  let { onBack }: Props = $props();

  const hotspot = $derived(globalState.hotspot);
  const isOn = $derived(hotspot?.state === HotspotState.on);
  const isBusy = $derived(hotspot?.state === HotspotState.inTransition);

  async function toggleHotspot() {
    if (!hotspot || isBusy) return;
    await invoke(SeelenCommand.SetNetworkHotspotState, { enabled: !isOn });
  }

  let showPassword = $state(false);
  let qrDataUrl = $state<string | null>(null);

  function escapeWifiField(value: string): string {
    return value.replace(/([\\;,:"])/g, "\\$1");
  }

  $effect(() => {
    const current = hotspot;
    if (!current || !current.ssid) {
      qrDataUrl = null;
      return;
    }

    const auth = current.passphrase ? "WPA" : "nopass";
    const payload = `WIFI:T:${auth};S:${escapeWifiField(current.ssid)};P:${
      current.passphrase ? escapeWifiField(current.passphrase) : ""
    };;`;

    QRCode.toDataURL(payload, { margin: 1, width: 176 }).then((url) => {
      qrDataUrl = url;
    });
  });

  function openHotspotSettings() {
    invoke(SeelenCommand.OpenFile, { path: "ms-settings:network-mobilehotspot" });
  }
</script>

<div class="hotspot-view">
  <div class="hotspot-view-header">
    <button data-skin="transparent" onclick={onBack}>
      <Icon iconName="IoArrowBack" />
    </button>
    <span class="hotspot-view-title">{$t("hotspot.title")}</span>
    <input
      type="checkbox"
      data-skin="switch"
      checked={isOn}
      disabled={isBusy}
      onchange={toggleHotspot}
    />
  </div>

  {#if hotspot}
    {#if qrDataUrl}
      <div class="hotspot-qr">
        <img src={qrDataUrl} alt={$t("hotspot.title")} />
      </div>
    {/if}

    <div class="hotspot-info">
      <div class="hotspot-info-row">
        <span class="hotspot-info-label">{$t("hotspot.ssid")}</span>
        <span class="hotspot-info-value">{hotspot.ssid || "-"}</span>
      </div>

      <div class="hotspot-info-row">
        <span class="hotspot-info-label">{$t("hotspot.password")}</span>
        {#if hotspot.passphrase}
          <div class="hotspot-info-password">
            <span class="hotspot-info-value">
              {showPassword ? hotspot.passphrase : "•".repeat(hotspot.passphrase.length)}
            </span>
            <button data-skin="transparent" onclick={() => (showPassword = !showPassword)}>
              <Icon iconName={showPassword ? "IoEyeOffOutline" : "IoEyeOutline"} />
            </button>
          </div>
        {:else}
          <span class="hotspot-info-value">{$t("hotspot.open_network")}</span>
        {/if}
      </div>

      <div class="hotspot-info-row">
        <span class="hotspot-info-label">{$t("hotspot.band")}</span>
        <span class="hotspot-info-value">{hotspot.band}</span>
      </div>
    </div>
  {/if}

  <div class="hotspot-footer">
    <button data-skin="transparent" onclick={openHotspotSettings}>
      {$t("hotspot.settings")}
    </button>
  </div>
</div>
