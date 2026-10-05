<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { outputVolumeIcon } from "libs/ui/utils";
  import { throttle } from "lodash";
  import type { MediaDevice } from "@seelen-ui/lib/types";
  import { onDestroy } from "svelte";
  import { t } from "../i18n";

  interface Props {
    output: MediaDevice;
    orientation: string;
  }

  let { output, orientation }: Props = $props();

  // svelte-ignore state_referenced_locally
  let currentVolume = $state(output.volume * 100);
  let isDragging = $state(false);

  $effect(() => {
    if (!isDragging) currentVolume = output.volume * 100;
  });

  const setVolumeThrottled = throttle((deviceId: string, level: number) => {
    invoke(SeelenCommand.SetVolumeLevel, { deviceId, sessionId: null, level: level / 100 }).catch(console.error);
  }, 100);

  onDestroy(() => setVolumeThrottled.cancel());

  function finishDrag() {
    setVolumeThrottled.flush();
    isDragging = false;
  }

  function toggleMute(deviceId: string) {
    invoke(SeelenCommand.MediaToggleMute, { deviceId, sessionId: null }).catch(console.error);
  }
</script>

<div class="volume">
  <button class="flyout-mute" onclick={() => toggleMute(output.id)}
    aria-label={$t("toggle_mute")} title={output.name} aria-pressed={output.muted}>
    <Icon iconName={outputVolumeIcon(output.muted, output.volume)} aria-hidden="true" />
  </button>
  <input
    type="range"
    data-skin="flat"
    data-orientation={orientation}
    aria-label={$t("volume")}
    style:--mac-level={`${currentVolume}%`}
    value={currentVolume}
    onpointerdown={(event) => {
      isDragging = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onpointerup={finishDrag}
    onpointercancel={finishDrag}
    onlostpointercapture={finishDrag}
    onchange={() => setVolumeThrottled.flush()}
    oninput={(e) => {
      currentVolume = Number(e.currentTarget.value);
      setVolumeThrottled(output.id, currentVolume);
    }}
    min={0}
    max={100}
    step={1}
  />
  <span class="flyout-value-label">{Math.round(currentVolume)}%</span>
</div>
