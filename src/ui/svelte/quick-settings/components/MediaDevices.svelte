<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { state } from "../state.svelte";
  import { throttle } from "lodash";
  import { t } from "../i18n";
  import { onDestroy } from "svelte";

  let defaultOutput = $derived(state.mediaOutputs.find((d) => d.isDefaultMultimedia));
  let defaultInput = $derived(state.mediaInputs.find((d) => d.isDefaultMultimedia));

  const setVolumeThrottled = throttle((deviceId: string, level: number) => {
    invoke(SeelenCommand.SetVolumeLevel, {
      deviceId,
      sessionId: null,
      level,
    }).catch(console.error);
  }, 100);

  onDestroy(() => setVolumeThrottled.cancel());

  function toggleMute(deviceId: string) {
    invoke(SeelenCommand.MediaToggleMute, { deviceId, sessionId: null }).catch(console.error);
  }
</script>

{#if defaultInput || defaultOutput}
  <span class="quick-settings-label">{$t("default_multimedia_volume")}</span>
{/if}

{#if defaultOutput}
  <div class="quick-settings-item">
    <button data-skin="transparent" onclick={() => toggleMute(defaultOutput!.id)}
      aria-pressed={defaultOutput.muted} aria-label={$t("output_mute")} title={defaultOutput.name}>
      <Icon iconName={defaultOutput.muted ? "IoVolumeMuteOutline" : "IoVolumeHighOutline"} />
    </button>
    <input
      type="range"
      data-skin="flat"
      value={defaultOutput.volume * 100}
      aria-label={$t("output_volume")}
      style:--mac-level={`${defaultOutput.volume * 100}%`}
      oninput={(e) => {
        setVolumeThrottled(defaultOutput.id, Number(e.currentTarget.value) / 100);
      }}
      onchange={() => setVolumeThrottled.flush()}
      min={0}
      max={100}
      step={1}
    />
    <span class="quick-settings-percentage">
      {Math.round(defaultOutput.volume * 100)}%
    </span>
  </div>
{/if}

{#if defaultInput}
  <div class="quick-settings-item">
    <button data-skin="transparent" onclick={() => toggleMute(defaultInput!.id)}
      aria-pressed={defaultInput.muted} aria-label={$t("input_mute")} title={defaultInput.name}>
      <Icon iconName={defaultInput.muted ? "BiMicrophoneOff" : "BiMicrophone"} />
    </button>
    <input
      type="range"
      data-skin="flat"
      value={defaultInput.volume * 100}
      aria-label={$t("input_volume")}
      style:--mac-level={`${defaultInput.volume * 100}%`}
      oninput={(e) => {
        setVolumeThrottled(defaultInput.id, Number(e.currentTarget.value) / 100);
      }}
      onchange={() => setVolumeThrottled.flush()}
      min={0}
      max={100}
      step={1}
    />
    <span class="quick-settings-percentage">
      {Math.round(defaultInput.volume * 100)}%
    </span>
  </div>
{/if}
