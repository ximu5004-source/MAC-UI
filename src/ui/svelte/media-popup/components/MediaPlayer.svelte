<script lang="ts">
  import type { MediaPlayer } from "@macui/lib/types";
  import { invoke, MacUiCommand } from "@macui/lib";
  import { Icon, FileIcon } from "libs/ui/svelte/components/Icon";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import MediaProgress from "./MediaProgress.svelte";
  import { t } from "../i18n";

  interface Props { session: MediaPlayer; }
  let { session }: Props = $props();
  const thumbnailSrc = $derived(session.thumbnail ? convertFileSrc(session.thumbnail) : null);
  let failedThumbnail = $state<string | null>(null);
  const artwork = $derived(thumbnailSrc && failedThumbnail !== thumbnailSrc ? thumbnailSrc : null);
  let pending = $state(false);
  let failed = $state(false);

  async function onClickBtn(cmd: MacUiCommand) {
    if (pending) return;
    pending = true;
    failed = false;
    try { await invoke(cmd, { id: session.umid }); }
    catch (error) { console.error(error); failed = true; }
    finally { pending = false; }
  }
</script>

<section class="media-session" aria-label={session.title.trim() || session.owner.name || $t("now_playing")} aria-busy={pending}>
  {#if artwork}<img class="media-session-blurred-thumbnail" src={artwork} alt="" />{/if}
  <div class="media-session-thumbnail-container">
    {#if artwork}
      <img class="media-session-thumbnail" src={artwork} alt="" onerror={(event) => failedThumbnail = event.currentTarget.getAttribute("src")} />
    {:else}
      <span class="media-session-artwork-placeholder" aria-hidden="true"><Icon iconName="IoMusicalNotes" /></span>
    {/if}
  </div>
  <div class="media-session-info">
    <h4 class="media-session-title" title={session.title}>{session.title.trim() || session.owner.name || $t("untitled")}</h4>
    <span class="media-session-author" title={session.author}>{session.author || session.owner.name}</span>
  </div>
  <FileIcon class="media-session-app-icon" umid={session.umid} title={session.owner.name} />
  <MediaProgress {session} />
  <div class="media-session-actions">
    <button data-skin="transparent" disabled={pending} onclick={() => onClickBtn(MacUiCommand.MediaPrev)} title={$t("previous")} aria-label={$t("previous")}><Icon iconName="IoPlaySkipBack" /></button>
    <button data-skin="transparent" class="media-session-play" disabled={pending} onclick={() => onClickBtn(MacUiCommand.MediaTogglePlayPause)} title={$t(session.playing ? "pause" : "play")} aria-label={$t(session.playing ? "pause" : "play")}><Icon iconName={session.playing ? "IoPause" : "IoPlay"} /></button>
    <button data-skin="transparent" disabled={pending} onclick={() => onClickBtn(MacUiCommand.MediaNext)} title={$t("next")} aria-label={$t("next")}><Icon iconName="IoPlaySkipForward" /></button>
  </div>
  {#if failed}<span class="media-session-error" role="status">{$t("control_failed")}</span>{/if}
</section>

<style>
  :global(#root) .media-session { background: var(--mac-card); color: var(--mac-ink); }
  :global(#root) .media-session-blurred-thumbnail { pointer-events: none; object-fit: cover; opacity: .12; }
  :global(#root) .media-session-thumbnail-container { width: 48px; height: 48px; }
  .media-session-artwork-placeholder { display: flex; align-items: center; justify-content: center; width: 48px; height: 48px; border-radius: 10px; background: var(--mac-selected); color: var(--mac-accent); }
  .media-session-artwork-placeholder :global(.macui-icon) { width: 26px; height: 26px; min-width: 26px; }
  :global(#root) .media-session-author { color: var(--mac-secondary); opacity: 1; }
  :global(#root) .media-session-actions .media-session-play { background: var(--mac-selected); color: var(--mac-accent); }
  .media-session-error { position: relative; grid-column: 1 / -1; font-size: 12px; color: var(--mac-secondary); }
</style>
