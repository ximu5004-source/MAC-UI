<script lang="ts">
  import { invoke, MacUiCommand } from "@macui/lib";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CommunicationNotification } from "@macui/lib/types";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import moment from "moment";
  import { t } from "../i18n";

  let { notification, onClose }: { notification: CommunicationNotification; onClose?: () => void } = $props();
  const appKeys = { WeChat: "messaging.wechat", WeCom: "messaging.wecom", DingTalk: "messaging.dingtalk", Feishu: "messaging.feishu" } as const;
  const appName = $derived($t(appKeys[notification.app]));
  let pending = $state(false);
  let error = $state(false);
  let iconFailed = $state(false);

  async function activate() {
    if (pending) return;
    pending = true;
    error = false;
    try {
      await invoke(MacUiCommand.ActivateCommunicationNotification, { id: notification.id });
      onClose?.();
    } catch (reason) {
      error = true;
      console.error("Failed to open messaging notification:", reason);
    } finally { pending = false; }
  }

  async function dismiss() {
    if (pending) return;
    pending = true;
    error = false;
    try {
      await invoke(MacUiCommand.DismissCommunicationNotification, { id: notification.id });
      onClose?.();
    } catch (reason) {
      error = true;
      console.error("Failed to dismiss messaging notification:", reason);
    } finally { pending = false; }
  }
</script>

<article class="notification communication-notification" aria-busy={pending}>
  <div class="notification-header">
    <div class="notification-header-info">
      {#if notification.iconPath && !iconFailed}
        <img class="notification-icon" src={convertFileSrc(notification.iconPath)} alt="" onerror={() => iconFailed = true} />
      {:else}
        <Icon class="notification-icon" iconName="IoChatbubblesOutline" />
      {/if}
      <div class="notification-app-name">{appName}</div>
      <div class="notification-time" title={moment(notification.date).format("LLL")}>
        {moment(notification.date).fromNow()}
      </div>
    </div>
    <button data-skin="transparent" onclick={dismiss} disabled={pending} aria-label={$t("dismiss")}>
      <Icon iconName="IoClose" />
    </button>
  </div>
  <button
    class="communication-content"
    data-skin="transparent"
    onclick={activate}
    disabled={pending}
    aria-label={$t("messaging.open", { app: appName })}
  >
    <div class="notification-body-content">
      {#if notification.source === "TrayActivity" || notification.source === "UnreadHint"}
        <p>{$t("messaging.new_message", { app: appName })}</p>
      {:else}
        {#if notification.title}<p class="communication-title">{notification.title}</p>{/if}
        {#if notification.body}<p>{notification.body}</p>{/if}
      {/if}
      <span class="communication-open">{$t("messaging.open", { app: appName })} <span aria-hidden="true">↗</span></span>
    </div>
  </button>
  {#if error}<p class="communication-error" role="status">{$t("messaging.action_failed")}</p>{/if}
</article>

<style>
  .notification-header-info { min-width: 0; flex: 1; }
  .notification-app-name { min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .notification-time { margin-left: auto; flex-shrink: 0; font-size: 11px; font-weight: 400; }
  .communication-notification :global(.notification-icon) { flex-shrink: 0; width: 18px; height: 18px; }
  .communication-content {
    display: block;
    width: 100%;
    padding: 4px 2px 2px;
    text-align: start;
    white-space: normal;
    line-height: 1.5;
  }
  .communication-content p { margin: 0 0 6px; overflow-wrap: anywhere; white-space: pre-wrap; }
  .communication-title { font-weight: 650; }
  .communication-open { display: inline-block; margin-top: 4px; font-size: 12px; }
  .communication-error { margin: 6px 0 0; font-size: 12px; }
  .communication-content:focus-visible { outline: 2px solid var(--mac-accent, var(--flyout-accent, #086bdc)); outline-offset: 3px; }
</style>
