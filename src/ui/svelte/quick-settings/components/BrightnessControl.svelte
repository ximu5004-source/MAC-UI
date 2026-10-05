<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { state } from "../state.svelte";
  import { brightnessIcon } from "libs/ui/utils";
  import { throttle } from "lodash";
  import { t } from "../i18n";
  import { onDestroy } from "svelte";

  const setBrightnessThrottled = throttle((instanceName: string, level: number) => {
    invoke(SeelenCommand.SetMonitorBrightness, { instanceName, level }).catch(console.error);
  }, 100);

  onDestroy(() => setBrightnessThrottled.cancel());
</script>

{#each state.brightness as brightness}
  <span class="quick-settings-label">{$t("brightness")}</span>
  <div class="quick-settings-item">
    <Icon iconName={brightnessIcon(brightness.currentBrightness)} />
    <input
      type="range"
      data-skin="flat"
      value={brightness.currentBrightness}
      aria-label={$t("brightness")}
      style:--mac-level={`${brightness.currentBrightness}%`}
      oninput={(e) => {
        setBrightnessThrottled(brightness.instanceName, Number(e.currentTarget.value));
      }}
      onchange={() => setBrightnessThrottled.flush()}
      min={brightness.availableLevels[0]}
      max={brightness.availableLevels.at(-1) ?? 100}
    />
    <span class="quick-settings-percentage">{brightness.currentBrightness}%</span>
  </div>
{/each}
