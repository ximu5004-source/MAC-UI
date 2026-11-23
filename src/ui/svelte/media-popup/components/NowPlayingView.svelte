<script lang="ts">
  import { globalState } from "../state.svelte";
  import { t } from "../i18n";
  import { selectNowPlayingSession } from "../model.ts";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import MediaPlayer from "./MediaPlayer.svelte";

  const sessions = $derived.by(() => {
    const primary = selectNowPlayingSession(globalState.sessions);
    return primary ? [primary, ...globalState.sessions.filter((session) => session !== primary)] : [];
  });
</script>

<div class="now-playing-view" role="dialog" aria-label={$t("now_playing")}>
  <h1 class="mac-panel-heading">{$t("now_playing")}</h1>
  {#if sessions.length > 0}
    <div class="media-control-session-list">
      {#each sessions as session (session.umid)}<MediaPlayer {session} />{/each}
    </div>
  {:else}
    <div class="now-playing-empty">
      <Icon iconName="IoMusicalNotes" />
      <strong>{$t("not_playing")}</strong>
      <span>{$t("start_playback")}</span>
    </div>
  {/if}
</div>

<style>
  .now-playing-view { display: flex; flex-direction: column; gap: 12px; }
  .now-playing-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px; min-height: 166px; padding: 20px; color: var(--mac-secondary); text-align: center; }
  .now-playing-empty :global(.macui-icon) { width: 32px; height: 32px; min-width: 32px; margin-bottom: 4px; color: var(--mac-accent); }
  .now-playing-empty strong { color: var(--mac-ink); font-size: 14px; font-weight: 600; }
  .now-playing-empty span { max-width: 260px; font-size: 12px; }
</style>
