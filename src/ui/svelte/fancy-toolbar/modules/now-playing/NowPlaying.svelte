<script lang="ts">
  import { invoke, MacUiCommand } from "@macui/lib";
  import { Alignment, FancyToolbarSide, type WidgetId } from "@macui/lib/types";
  import type { createSortable } from "@dnd-kit/svelte/sortable";
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { toPhysicalPixels } from "libs/ui/react/utils/index.ts";
  import { nowPlayingState } from "./state.svelte.ts";
  import { settingsState } from "../../state/settings.svelte.ts";
  import { t } from "../../i18n";
  import { isPrimaryToolbarClick } from "../../activation.ts";
  import { CssHandled } from "libs/ui/svelte/utils/animations.ts";

  interface Props {
    id: string;
    pluginId: string;
    sortable?: ReturnType<typeof createSortable> | null;
    style?: string | null;
    oncontextmenu: (event: MouseEvent) => void;
  }

  let { id, pluginId, sortable = null, style = null, oncontextmenu }: Props = $props();
  const noopAttach = () => {};
  onMount(() => nowPlayingState.retain());

  const session = $derived(nowPlayingState.session);
  const title = $derived(session?.title.trim() || session?.owner.name || $t("media.now_playing"));
  const thumbnail = $derived(session?.thumbnail ? convertFileSrc(session.thumbnail) : null);
  let failedThumbnail = $state<string | null>(null);
  let opening = $state(false);
  const status = $derived(nowPlayingState.unavailable ? $t("media.unavailable") : session
    ? $t(session.playing ? "media.playing" : "media.paused") : $t("media.not_playing"));
  const label = $derived(`${$t("media.now_playing")} · ${session ? `${title} · ` : ""}${status}`);

  async function openPlayer(event: MouseEvent) {
    event.stopPropagation();
    if (!isPrimaryToolbarClick(event, sortable?.isDragging)) return;
    if (opening) return;
    const rect = event.currentTarget instanceof HTMLElement ? event.currentTarget.getBoundingClientRect() : null;
    if (!rect) return;
    opening = true;
    try {
      const isTop = settingsState.position === FancyToolbarSide.Top;
      const rootRect = document.getElementById("root")!.getBoundingClientRect();
      await invoke(MacUiCommand.TriggerWidget, {
        payload: {
          id: "@macui/media-popup" as WidgetId,
          customArgs: {
            view: "now-playing",
            __macuiPopupToggleSource: `music:${id}:${settingsState.widgetRect.left}:${settingsState.widgetRect.top}`,
          },
          desiredPosition: {
            x: settingsState.widgetRect.left + toPhysicalPixels(rect.left + rect.width / 2),
            y: settingsState.widgetRect.top + toPhysicalPixels(isTop ? rootRect.bottom : rootRect.top),
          },
          alignX: Alignment.Center,
          alignY: isTop ? Alignment.Start : Alignment.End,
        },
      });
    } catch (error) { console.error(error); }
    finally { opening = false; }
  }
</script>

<button
  {id}
  {style}
  {@attach sortable?.attach ?? noopAttach}
  data-plugin-id={pluginId}
  data-dragging={sortable?.isDragging}
  class="toolbar-now-playing"
  class:has-session={!!session}
  class:is-playing={!!session?.playing}
  title={label}
  aria-label={label}
  aria-haspopup="dialog"
  aria-busy={opening}
  onclick={openPlayer}
  {oncontextmenu}
  transition:CssHandled|global={{ enabled: () => !!sortable && !sortable.isDragging }}
>
  {#if thumbnail && failedThumbnail !== thumbnail}
    <img class="now-playing-artwork" src={thumbnail} alt="" onerror={(event) => failedThumbnail = event.currentTarget.getAttribute("src")} />
  {:else}
    <span class="now-playing-icon" aria-hidden="true"><Icon iconName="IoMusicalNotes" /></span>
  {/if}
  {#if session}
    <span class="now-playing-title">{title}</span>
    {#if session.playing}
      <span class="now-playing-wave" aria-hidden="true"><i></i><i></i><i></i><i></i></span>
    {:else}
      <span class="now-playing-paused" aria-hidden="true"><Icon iconName="IoPause" /></span>
    {/if}
  {/if}
</button>

<style>
  .toolbar-now-playing {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 1 auto;
    min-width: var(--config-item-size, 28px);
    max-width: 184px;
    height: var(--config-item-size, 28px);
    gap: 6px;
    padding: 0 6px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
    transition: background-color 140ms ease, border-color 140ms ease;
  }
  .toolbar-now-playing.has-session {
    background: light-dark(rgb(255 255 255 / 25%), rgb(255 255 255 / 8%));
    border-color: light-dark(rgb(0 0 0 / 9%), rgb(255 255 255 / 13%));
  }
  .toolbar-now-playing:hover { background: var(--macui-std-ui-hover-overlay); }
  .toolbar-now-playing:active { background: var(--macui-std-ui-pressed-overlay); }
  .toolbar-now-playing:focus-visible { outline: 2px solid var(--macui-std-ui-color); outline-offset: -2px; }
  .now-playing-artwork { flex-shrink: 0; width: 18px; height: 18px; object-fit: cover; border-radius: 4px; }
  .now-playing-icon, .now-playing-paused { flex-shrink: 0; display: flex; align-items: center; justify-content: center; }
  .now-playing-icon :global(.macui-icon) { width: 16px; height: 16px; min-width: 16px; }
  .now-playing-paused :global(.macui-icon) { width: 12px; height: 12px; min-width: 12px; }
  .now-playing-title { min-width: 0; max-width: 124px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .now-playing-wave { flex-shrink: 0; display: flex; align-items: center; gap: 2px; height: 14px; }
  .now-playing-wave i { width: 2px; height: 12px; border-radius: 2px; background: currentColor; transform: scaleY(.3); animation: now-playing-pulse 1s ease-in-out infinite alternate; }
  .now-playing-wave i:nth-child(2) { animation-delay: -.35s; }
  .now-playing-wave i:nth-child(3) { animation-delay: -.65s; }
  .now-playing-wave i:nth-child(4) { animation-delay: -.2s; }
  @keyframes now-playing-pulse { to { transform: scaleY(1); } }
  @media (max-width: 1100px) {
    .now-playing-title { display: none; }
    .toolbar-now-playing { max-width: 58px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .toolbar-now-playing { transition: none; }
    .now-playing-wave i { animation: none; }
    .now-playing-wave i:nth-child(2n) { transform: scaleY(.75); }
  }
</style>
