<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { throttle } from "lodash";
  import { t } from "../i18n";
  import { onDestroy } from "svelte";

  interface Props {
    value: number;
    icon: string;
    deviceId: string;
    sessionName?: string;
    sessionId?: string;
    onRightAction?: () => void;
    withPercentage?: boolean;
    mutedIcon?: string;
    muted?: boolean;
    label?: string;
  }

  let {
    value,
    icon,
    deviceId,
    sessionName,
    sessionId,
    onRightAction,
    withPercentage = false,
    mutedIcon,
    muted = false,
    label,
  }: Props = $props();

  let internalValue = $state(0);
  let isDragging = $state(false);

  $effect(() => {
    if (!isDragging) internalValue = value * 100;
  });

  const onExternalChange = throttle((value: number) => {
    invoke(SeelenCommand.SetVolumeLevel, {
      deviceId,
      sessionId: sessionId || null,
      level: value / 100,
    }).catch(console.error);
  }, 100);

  onDestroy(() => onExternalChange.cancel());

  function finishDrag() {
    onExternalChange.flush();
    isDragging = false;
  }

  async function toggleMute() {
    await invoke(SeelenCommand.MediaToggleMute, {
      deviceId,
      sessionId: sessionId || null,
    });
  }
</script>

<div class="media-control-volume">
  <button data-skin="transparent" onclick={toggleMute} title={sessionName}
    aria-label={`${label || sessionName || $t("device.volume")} — ${$t("toggle_mute")}`} aria-pressed={muted}>
    <Icon iconName={(muted && mutedIcon ? mutedIcon : icon) as any} />
  </button>

  <input
    type="range"
    data-skin="flat"
    value={internalValue}
    aria-label={label || sessionName || $t("device.volume")}
    style:--mac-level={`${internalValue}%`}
    min={0}
    max={100}
    step={1}
    onpointerdown={(event) => {
      isDragging = true;
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onpointerup={finishDrag}
    onpointercancel={finishDrag}
    onlostpointercapture={finishDrag}
    onchange={() => onExternalChange.flush()}
    oninput={(e) => {
      internalValue = Number(e.currentTarget.value);
      onExternalChange(internalValue);
    }}
  />

  {#if onRightAction}
    <button data-skin="transparent" onclick={onRightAction} title={$t("device.mixer")} aria-label={$t("device.mixer")}>
      <Icon iconName="RiEqualizerLine" />
    </button>
  {/if}
</div>
