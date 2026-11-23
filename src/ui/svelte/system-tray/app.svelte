<script lang="ts">
  import { onMount } from "svelte";
  import { SystrayIconAction, type SysTrayIcon } from "@macui/lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { invoke, MacUiCommand, Widget } from "@macui/lib";
  import { MissingIcon } from "libs/ui/svelte/components/Icon";
  import { t } from "libs/ui/svelte/utils/i18n";
  import { state as tray } from "./state.svelte";
  import { allTrayItems, trayIconKey, trayTileKey, trayIconLabel } from "./trayPresentation";

  const items = $derived(allTrayItems(tray.items));
  let actionError = $state("");
  let failedIcons = $state(new Set<string>());

  onMount(() => { void Widget.self.ready().catch(console.error); });

  function label(item: SysTrayIcon) {
    return trayIconLabel(item) || $t("tray.unnamed");
  }

  function imageKey(item: SysTrayIcon) {
    return trayIconKey(item) + ":" + item.icon_path + ":" + item.icon_image_hash;
  }

  async function sendAction(item: SysTrayIcon, action: SystrayIconAction) {
    actionError = "";
    try {
      await invoke(MacUiCommand.SendSystemTrayIconAction, { id: item.stable_id, action });
      if (action === SystrayIconAction.LeftClick || action === SystrayIconAction.LeftDoubleClick) {
        tray.acknowledgeActivity(item);
      }
    } catch (error) {
      console.error(error);
      actionError = $t("tray.action_failed", { name: label(item) });
    }
  }

  function click(event: MouseEvent, item: SysTrayIcon) {
    event.preventDefault();
    event.stopPropagation();
    if (event.button !== 0) return;
    // Keep native single/double-click semantics, without dispatching the second single click.
    if (event.detail !== 2) void sendAction(item, SystrayIconAction.LeftClick);
  }
</script>

<svelte:window onkeydown={(event) => {
  if (event.key === "Escape") void Widget.self.hide().catch(console.error);
  if (event.key === "F5") { event.preventDefault(); actionError = ""; void tray.refresh(); }
}} />

<div class="macui-std-popover mac-panel mac-frosted-surface system-tray" aria-label={$t("tray.title")}>
  {#if tray.error || actionError}
    <p class="tray-feedback" role="alert">{actionError || $t("tray.load_failed")}</p>
  {/if}

  <div class="tray-list" aria-label={$t("tray.all_apps")} aria-busy={tray.loading}>
    {#each items as item (trayTileKey(item))}
      <button class="system-tray-item"
        class:needs-attention={tray.isFlashing(item)}
        aria-label={label(item)} title={label(item)} data-skin="transparent"
        onclick={(event) => click(event, item)}
        ondblclick={(event) => { event.preventDefault(); event.stopPropagation(); if (event.button === 0) void sendAction(item, SystrayIconAction.LeftDoubleClick); }}
        onauxclick={(event) => { event.preventDefault(); event.stopPropagation(); if (event.button === 1) void sendAction(item, SystrayIconAction.MiddleClick); }}
        oncontextmenu={(event) => { event.preventDefault(); event.stopPropagation(); void sendAction(item, SystrayIconAction.RightClick); }}>
        <div class="system-tray-item-icon-box">
          {#if item.icon_path && !failedIcons.has(imageKey(item))}
            <img class="system-tray-item-icon" alt="" width="20" height="20"
              src={convertFileSrc(item.icon_path) + "?hash=" + (item.icon_image_hash || "null")}
              onerror={() => { failedIcons = new Set([...failedIcons, imageKey(item)]); }} />
          {:else}
            <MissingIcon class="system-tray-item-icon" />
          {/if}
        </div>
        {#if tray.hasNotification(item) || tray.hasActivity(item)}
          <span class="tray-dot" aria-hidden="true"></span>
        {/if}
      </button>
    {:else}
      <p class="tray-empty" role="status">
        {$t(tray.loading ? "tray.loading" : tray.error ? "tray.retry_hint" : "tray.empty")}
      </p>
    {/each}
  </div>
</div>

<style>
  :global(#root) .macui-std-popover.mac-panel.system-tray { border-radius: 28px; }
  /* Using vh here creates a shrinking feedback loop with autoSizeByContent. */
  .system-tray {
    width: var(--tray-width, 360px);
    max-height: var(--tray-max-height, 600px);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }

  .tray-list { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(44px, 100%), 1fr)); align-content: start; gap: 4px; flex: 1 1 auto; min-height: 0; overflow-y: auto; overflow-x: hidden; scrollbar-width: thin; scrollbar-color: var(--mac-track) transparent; padding: 2px; }
  .system-tray .system-tray-item { position: relative; min-width: 0; border: 1px solid transparent; cursor: pointer; }
  .system-tray-item:hover { background: color-mix(in srgb, currentColor 9%, transparent); }
  .tray-dot { position: absolute; bottom: 5px; right: 7px; width: 6px; height: 6px; border-radius: 50%; background: #bd6417; }
  .system-tray-item.needs-attention {
    border-color: var(--color-amber-700, #a65e0b);
    background: color-mix(in srgb, var(--color-amber-500, #e1a12c) 14%, transparent);
  }
  .system-tray-item.needs-attention .system-tray-item-icon-box { animation: tray-attention 1.6s ease-in-out infinite; }
  @keyframes tray-attention { 0%, 100% { opacity: 0.45; } 50% { opacity: 1; } }
  @media (prefers-reduced-motion: reduce) {
    .system-tray-item.needs-attention .system-tray-item-icon-box { animation: none; }
  }
  button:focus-visible { outline: 2px solid var(--macui-std-fg-color); outline-offset: -2px; }
  .tray-empty, .tray-feedback { grid-column: 1 / -1; margin: 0; padding: 12px 8px; font-size: 12px; line-height: 1.5; }
  .tray-feedback { flex-shrink: 0; border: 1px solid var(--color-red-700); border-radius: 8px; }
</style>
