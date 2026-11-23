<script lang="ts">
  import { globalState } from "./state.svelte";
  import { Widget, invoke, MacUiCommand } from "@macui/lib";
  import { NotificationsMode } from "@macui/lib/types";
  import { t } from "./i18n/index.ts";
  import Notification from "./components/Notification.svelte";
  import CommunicationNotification from "./components/CommunicationNotification.svelte";
  import { Icon } from "libs/ui/svelte/components/Icon";

  $effect(() => {
    Widget.getCurrent().ready();
  });

  async function handleClearAll() {
    // Clear the independent bridge even when Windows notification access is unavailable.
    const results = await Promise.allSettled([
      invoke(MacUiCommand.NotificationsCloseAll),
      invoke(MacUiCommand.ClearCommunicationNotifications),
    ]);
    for (const result of results) {
      if (result.status === "rejected") console.error("Failed to clear notifications:", result.reason);
    }
  }

  const isDndActive = $derived(globalState.focusAssistMode !== NotificationsMode.All);
  async function toggleDnd() {
    await invoke(MacUiCommand.SetNotificationsMode, {
      mode: isDndActive ? NotificationsMode.All : NotificationsMode.AlarmsOnly,
    });
  }

  async function handleOpenSettings() {
    try {
      await invoke(MacUiCommand.OpenFile, {
        path: "ms-settings:notifications",
      });
    } catch (error) {
      console.error("Failed to open notification settings:", error);
    }
  }

  let closing = false;
  async function handleEscape(event: KeyboardEvent) {
    if (event.key !== "Escape" || event.repeat || closing) return;
    event.preventDefault();
    closing = true;
    try { await Widget.getCurrent().hide(); }
    catch (error) { console.error("Failed to hide notifications popup:", error); }
    finally { closing = false; }
  }
</script>

<svelte:window onkeydown={handleEscape} />

<div class="macui-std-popover mac-panel mac-frosted-surface notifications-popup">
  <div class="notifications-popup-header">
    <span>{$t("title")}</span>
    <button
      data-skin={isDndActive ? "solid" : "default"}
      onclick={toggleDnd}
      aria-label={$t("dnd")}
      aria-pressed={isDndActive}
      disabled={!globalState.modeAvailable}
    >
      <Icon iconName={isDndActive ? "IoMoon" : "IoMoonOutline"} />
    </button>
    <button data-skin="default" onclick={handleClearAll}>
      {$t("clear")}
    </button>
  </div>

  <div class="notifications-popup-body">
    {#if globalState.nativeUnavailable}
      <p class="notifications-access-status" role="status">{$t("messaging.windows_unavailable")}</p>
    {/if}
    {#if globalState.communicationUnavailable}
      <p class="notifications-access-status" role="status">{$t("messaging.bridge_unavailable")}</p>
    {/if}
    {#each globalState.notifications as item (item.key)}
      {#if item.source === "windows"}
        <Notification notification={item.notification} />
      {:else}
        <CommunicationNotification notification={item.notification} />
      {/if}
    {/each}

    {#if globalState.notifications.length === 0}
      <div class="notifications-popup-empty">
        <p>{$t("empty")}</p>
      </div>
    {/if}
  </div>

  <div class="notifications-popup-footer">
    <button data-skin="transparent" onclick={handleOpenSettings}>
      {$t("settings")}
    </button>
  </div>
</div>

<style>
  :global(#root) .macui-std-popover.mac-panel.notifications-popup { border-radius: 28px; }
  .notifications-popup-body {
    flex: 1 1 auto;
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: var(--mac-track) transparent;
  }
  :global(#root) .notifications-popup :global(.notification) { flex-shrink: 0; }
  .notifications-popup-empty { flex: 1 0 160px; }
  .notifications-access-status { margin: 0; padding: 4px 2px; font-size: 12px; line-height: 1.5; }
</style>
