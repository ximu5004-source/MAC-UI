<script lang="ts">
  import { HotspotState } from "@seelen-ui/lib/types";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { globalState } from "../state.svelte";
  import { t } from "../i18n";

  const hotspot = $derived(globalState.hotspot!);
  const isOn = $derived(hotspot.state === HotspotState.on);

  function openHotspotView() {
    globalState.view = "hotspot";
  }
</script>

<div
  class="hotspot-entry"
  onclick={openHotspotView}
  role="button"
  tabindex="0"
  onkeydown={(e) => {
    if (e.key === "Enter" || e.key === " ") openHotspotView();
  }}
>
  <Icon iconName="MdWifiTethering" />
  <span class="hotspot-entry-label">{hotspot.ssid || $t("hotspot.title")}</span>
  <span class="hotspot-entry-status" class:hotspot-entry-status-on={isOn}>
    {isOn ? $t("hotspot.on") : $t("hotspot.off")}
  </span>
  <Icon iconName="IoChevronForward" />
</div>
