<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { brightnessIcon } from "libs/ui/utils";
  import { throttle } from "lodash";
  import type { MonitorBrightness } from "@seelen-ui/lib/types";
  import { onDestroy } from "svelte";
  import { t } from "../i18n";

  interface Props {
    brightness: MonitorBrightness;
    orientation: string;
  }

  let { brightness, orientation }: Props = $props();

  // svelte-ignore state_referenced_locally
  let currentBrightness = $state(brightness.currentBrightness);
  let isDragging = $state(false);

  $effect(() => {
    if (!isDragging) currentBrightness = brightness.currentBrightness;
  });

  const setBrightnessThrottled = throttle((instanceName: string, level: number) => {
    invoke(SeelenCommand.SetMonitorBrightness, { instanceName, level }).catch(console.error);
  }, 100);

  onDestroy(() => setBrightnessThrottled.cancel());

  function finishDrag() {
    setBrightnessThrottled.flush();
    isDragging = false;
  }
</script>

<div class="brightness">
  <Icon iconName={brightnessIcon(currentBrightness)} />
  <input
    type="range"
    data-skin="flat"
    data-orientation={orientation}
    aria-label={$t("brightness")}
    style:--mac-level={`${currentBrightness}%`}
    value={currentBrightness}
    onpointerdown={(event) => {
      isDragging = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onpointerup={finishDrag}
    onpointercancel={finishDrag}
    onlostpointercapture={finishDrag}
    onchange={() => setBrightnessThrottled.flush()}
    oninput={(e) => {
      currentBrightness = Number(e.currentTarget.value);
      setBrightnessThrottled(brightness.instanceName, currentBrightness);
    }}
    min={brightness.availableLevels[0]}
    max={brightness.availableLevels.at(-1) ?? 100}
  />
  <span class="flyout-value-label">{Math.round(currentBrightness)}%</span>
</div>
