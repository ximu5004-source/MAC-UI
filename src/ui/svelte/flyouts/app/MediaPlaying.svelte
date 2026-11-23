<script lang="ts">
  import { invoke, MacUiCommand } from "@macui/lib";
  import { Icon, SpecificIcon } from "libs/ui/svelte/components/Icon";
  import { nanosecondsToPlayingTime } from "libs/ui/utils";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { MediaPlayer } from "@macui/lib/types";

  interface Props {
    playing: MediaPlayer;
  }

  let { playing }: Props = $props();
</script>

<div class="player">
  <div class="player-thumbnail-container">
    {#if playing.thumbnail}
      <img class="player-thumbnail" src={convertFileSrc(playing.thumbnail)} alt="" />
    {:else}
      <SpecificIcon name="defaultPlayerThumbnail" />
    {/if}
  </div>

  <div class="player-info">
    <div class="player-title">{playing.title}</div>
    <div class="player-author">{playing.author}</div>
    <div class="player-timeline">
      <span>{nanosecondsToPlayingTime(playing.timeline.position as any)}</span>
      <span>/</span>
      <span>{nanosecondsToPlayingTime(playing.timeline.end as any)}</span>
    </div>
  </div>

  <div class="player-controls">
    <button
      data-skin="transparent"
      onclick={() => invoke(MacUiCommand.MediaPrev, { id: playing.umid })}
    >
      <Icon iconName="IoPlaySkipBack" />
    </button>
    <button
      data-skin="transparent"
      onclick={() => invoke(MacUiCommand.MediaTogglePlayPause, { id: playing.umid })}
    >
      <Icon iconName={playing.playing ? "IoPause" : "IoPlay"} />
    </button>
    <button
      data-skin="transparent"
      onclick={() => invoke(MacUiCommand.MediaNext, { id: playing.umid })}
    >
      <Icon iconName="IoPlaySkipForward" />
    </button>
  </div>

  <progress
    class="player-progress"
    value={playing.timeline.position as any}
    max={playing.timeline.end as any}
  >
    <span>{nanosecondsToPlayingTime(playing.timeline.position as any)}</span>
    <span>/</span>
    <span>{nanosecondsToPlayingTime(playing.timeline.end as any)}</span>
  </progress>
</div>
