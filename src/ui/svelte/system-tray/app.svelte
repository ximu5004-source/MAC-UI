<script lang="ts">
  import { onMount } from "svelte";
  import { SystrayIconAction, type SysTrayIcon } from "@seelen-ui/lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon";
  import { t } from "libs/ui/svelte/utils/i18n";
  import { state as tray } from "./state.svelte";
  import { allTrayItems, trayIconKey, trayIconLabel } from "./trayPresentation";

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
      await invoke(SeelenCommand.SendSystemTrayIconAction, { id: item.stable_id, action });
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
}} />

<div class="slu-std-popover mac-panel mac-frosted-surface system-tray" aria-label={$t("tray.title")}>
  <header class="tray-header">
    <h1>{$t("tray.title")}</h1>
    <span class="tray-count" role="status">{$t("tray.count", { count: String(items.length) })}</span>
    <button class="tray-refresh" data-skin="transparent" disabled={tray.loading}
      title={$t("tray.refresh")} aria-label={$t("tray.refresh")}
      onclick={() => { actionError = ""; void tray.refresh(); }}>
      <Icon name="TbRefresh" aria-hidden="true" />
    </button>
  </header>
  <p class="tray-description">{$t("tray.description")}</p>

  {#if tray.error || actionError}
    <p class="tray-feedback" role="alert">{actionError || $t("tray.load_failed")}</p>
  {/if}

  <div class="tray-list" aria-label={$t("tray.all_apps")} aria-busy={tray.loading}>
    {#each items as item (trayIconKey(item))}
      <button class="system-tray-item" class:communication-app={!!tray.name(item)}
        title={item.notification || label(item)} data-skin="transparent"
        onclick={(event) => click(event, item)}
        ondblclick={(event) => { event.preventDefault(); event.stopPropagation(); if (event.button === 0) void sendAction(item, SystrayIconAction.LeftDoubleClick); }}
        onauxclick={(event) => { event.preventDefault(); event.stopPropagation(); if (event.button === 1) void sendAction(item, SystrayIconAction.MiddleClick); }}
        oncontextmenu={(event) => { event.preventDefault(); event.stopPropagation(); void sendAction(item, SystrayIconAction.RightClick); }}>
        <div class="system-tray-item-icon-box">
          {#if item.icon_path && !failedIcons.has(imageKey(item))}
            <img class="system-tray-item-icon" alt="" width="24" height="24"
              src={convertFileSrc(item.icon_path) + "?hash=" + (item.icon_image_hash || "null")}
              onerror={() => { failedIcons = new Set([...failedIcons, imageKey(item)]); }} />
          {:else}
            <MissingIcon class="system-tray-item-icon" />
          {/if}
        </div>
        <span class="system-tray-item-label">
          <span class="tray-app-name">{label(item)}</span>
          {#if tray.name(item)}
            <small class:has-notification={tray.hasNotification(item)}>
              {$t(tray.hasNotification(item) ? "communication.notification" : tray.hasActivity(item) ? "communication.activity" : "communication.running")}
            </small>
          {:else if !item.is_visible}
            <small>{$t("tray.hidden_icon")}</small>
          {/if}
        </span>
      </button>
    {:else}
      <p class="tray-empty" role="status">
        {$t(tray.loading ? "tray.loading" : tray.error ? "tray.retry_hint" : "tray.empty")}
      </p>
    {/each}
  </div>
</div>

<style>
  :global(#root) .slu-std-popover.mac-panel.system-tray { border-radius: 28px; }
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
  .tray-header { display: flex; align-items: center; gap: 8px; padding: 4px 6px 0; flex-shrink: 0; }
  h1 { margin: 0; font-size: 14px; font-weight: 650; }
  .tray-count { margin-left: auto; font-size: 12px; color: color-mix(in srgb, var(--slu-std-fg-color) 80%, transparent); }
  .tray-refresh { display: grid; place-items: center; width: 30px; height: 30px; padding: 4px; border-radius: 7px; cursor: pointer; }
  .tray-refresh:disabled { cursor: wait; opacity: 0.5; }
  .tray-description { margin: 0; padding: 0 6px 6px; font-size: 12px; line-height: 1.5; flex-shrink: 0; }
  .tray-list { display: flex; flex-direction: column; gap: 4px; flex: 1 1 auto; min-height: 0; overflow-y: auto; overflow-x: hidden; scrollbar-width: thin; scrollbar-color: var(--mac-track) transparent; padding: 2px; }
  .system-tray-item { display: flex; align-items: center; gap: 10px; flex: 0 0 auto; min-height: 44px; width: 100%; padding: 7px 8px; border: 1px solid transparent; border-radius: 9px; cursor: pointer; }
  .system-tray-item.communication-app { border-color: color-mix(in srgb, var(--slu-std-fg-color) 18%, transparent); }
  .system-tray-item-icon-box { width: 28px; height: 28px; flex-shrink: 0; display: grid; place-items: center; }
  .system-tray-item-icon { width: 24px; height: 24px; object-fit: contain; }
  .system-tray-item-label { flex: 1; min-width: 0; text-align: left; font-size: 13px; font-weight: 500; }
  .tray-app-name { display: block; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .system-tray-item-label small { display: block; font-weight: 400; font-size: 12px; color: color-mix(in srgb, var(--slu-std-fg-color) 80%, transparent); }
  .system-tray-item-label small.has-notification { color: var(--color-red-700); font-weight: 600; }
  button:focus-visible { outline: 2px solid var(--slu-std-fg-color); outline-offset: -2px; }
  .tray-empty, .tray-feedback { margin: 0; padding: 12px 8px; font-size: 12px; line-height: 1.5; }
  .tray-feedback { flex-shrink: 0; border: 1px solid var(--color-red-700); border-radius: 8px; }
</style>
