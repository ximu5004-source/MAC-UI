<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { SystrayIconAction, type SysTrayIcon } from "@seelen-ui/lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { MissingIcon } from "libs/ui/svelte/components/Icon";
  import { communicationTray as tray } from "libs/ui/svelte/runes/CommunicationTray.svelte";
  import { t } from "../i18n";

  function status(item: SysTrayIcon) {
    return $t(tray.hasNotification(item) ? "communication.notification" : tray.hasActivity(item)
      ? "communication.activity" : "communication.running");
  }
  async function activate(item: SysTrayIcon) {
    try {
      await invoke(SeelenCommand.SendSystemTrayIconAction, { id: item.stable_id, action: SystrayIconAction.LeftDoubleClick });
      tray.acknowledgeActivity(item);
    } catch (error) { console.error(error); }
  }
</script>

{#if tray.communicationItems.length > 0}
  <div class="communication-tray" role="group" aria-label={$t("communication.label")}>
    {#each tray.communicationItems.slice(0, 8) as item (JSON.stringify(item.stable_id))}
      <button
        class="communication-tray-app"
        title={`${tray.name(item)} · ${status(item)}\n${item.notification || item.tooltip}`}
        aria-label={`${tray.name(item)} · ${status(item)}`}
        onclick={(event) => { event.stopPropagation(); if (event.button === 0) void activate(item); }}
        oncontextmenu={(event) => {
          event.preventDefault(); event.stopPropagation();
          void invoke(SeelenCommand.SendSystemTrayIconAction, { id: item.stable_id, action: SystrayIconAction.RightClick }).catch(console.error);
        }}
      >
        {#if item.icon_path}
          <img src={`${convertFileSrc(item.icon_path)}?hash=${item.icon_image_hash}`} alt="" />
        {:else}<MissingIcon />{/if}
        <span class="status-dot" class:has-notification={tray.hasNotification(item)} class:has-activity={!tray.hasNotification(item) && tray.hasActivity(item)}></span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .communication-tray { display: flex; align-items: center; gap: 3px; padding: 0 5px; border-inline-end: 1px solid rgb(128 128 128 / 0.25); }
  .communication-tray-app { position: relative; display: grid; place-items: center; width: 28px; height: 26px; border: 1px solid transparent; border-radius: 8px; background: transparent; cursor: pointer; }
  .communication-tray-app:hover, .communication-tray-app:focus-visible { background: rgb(255 255 255 / 0.35); border-color: rgb(95 142 205 / 0.65); outline: 2px solid rgb(95 142 205 / 0.6); }
  .communication-tray-app img { width: 18px; height: 18px; object-fit: contain; }
  .status-dot { position: absolute; width: 6px; height: 6px; border: 1px solid white; border-radius: 50%; bottom: 1px; right: 1px; background: #38844b; }
  .status-dot.has-notification { width: 8px; height: 8px; background: #db343f; }
  .status-dot.has-activity { width: 8px; height: 8px; background: #c77b13; }
</style>
