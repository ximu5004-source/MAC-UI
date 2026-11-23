<script lang="ts">
  import { invoke, MacUiCommand } from "@macui/lib";
  import { WegMiddleClickAction, type UserAppWindow } from "@macui/lib/types";
  import { FileIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import { t } from "../../i18n/index.ts";
  import type { AppOrFileWegItem } from "../../types.ts";
  import { settingsState } from "../../state/settings.svelte.ts";
  import { windowsState, focused } from "../../state/windows.svelte.ts";
  import { notifications } from "../../state/getters.svelte.ts";
  import { activateItem, getUserApplicationContextMenu, launchItem } from "../../appMenu.ts";
  import { triggerPreviewWidget } from "../../previewWidget.ts";
  import { onDestroy } from "svelte";
  import { dockIsDragging } from "../../state/hidden.svelte.ts";
  import { event as events } from "@macui/lib/tauri";
  import { CssHandled } from "libs/ui/svelte/utils/animations.ts";
  import { isDockPrimaryClick } from "../../activation.ts";
  import { captureDockTarget } from "../../dockTarget";
  import { registerWindowAnimationTarget } from "../../animationTargets";

  interface Props {
    item: AppOrFileWegItem;
    windows: UserAppWindow[];
    isOverlay?: boolean;
  }

  let { item, windows, isOverlay = false }: Props = $props();

  const settings = $derived(settingsState.value as any);
  const notificationsCount = $derived(
    notifications.value.filter((n: any) => n.appUmid === item.umid).length,
  );
  const itemLabel = $derived(
    settings?.showWindowTitle && windows.length ? windows[0]!.title : null,
  );
  const isFocused = $derived(windows.some((w) => w.hwnd === focused.value?.hwnd));

  let itemEl: HTMLDivElement | null = $state(null);
  let activating = $state(false);
  let activationError = $state(false);
  let shortcutIsApp = $state(false);
  const isApp = $derived(Boolean(item.umid) || /\.(exe|appref-ms)$/i.test(item.path) || shortcutIsApp);

  $effect(() => {
    const hwnds = windows.map((window) => window.hwnd);
    if (itemEl && !isOverlay) return registerWindowAnimationTarget(itemEl, hwnds);
    return undefined;
  });

  $effect(() => {
    const path = item.path;
    shortcutIsApp = false;
    if (!/\.lnk$/i.test(path)) return;
    let disposed = false;
    invoke(MacUiCommand.IsApplicationFile, { path }).then((value) => {
      if (!disposed) shortcutIsApp = value;
    }).catch(console.error);
    return () => { disposed = true; };
  });

  let hoverTimer: ReturnType<typeof setTimeout>;
  let hoverRevision = 0;

  function cancelHover() {
    hoverRevision++;
    clearTimeout(hoverTimer);
  }

  function onEnter(event: PointerEvent) {
    cancelHover();
    if (event.pointerType !== "mouse" || event.buttons || isOverlay || !windows.length) return;
    const revision = hoverRevision;
    const stillHere = () => revision === hoverRevision && !!itemEl?.matches(":hover") &&
      !dockIsDragging.value && windows.length > 0;
    hoverTimer = setTimeout(() => {
      if (itemEl && stillHere()) void triggerPreviewWidget(itemEl, windows, stillHere).catch(console.error);
    }, 260);
  }

  function onPress() {
    cancelHover();
    void events.emit("macui:dismiss-dock-hover").catch(console.error);
  }

  onDestroy(cancelHover);

  async function onClick(event?: MouseEvent) {
    if (event && !isDockPrimaryClick(event)) return;
    if (activating) return;
    activationError = false;
    if (windows.length > 1) {
      void triggerPreviewWidget(itemEl!, windows).catch(console.error);
      return;
    }

    const win = windows[0];
    if (!win) {
      activating = true;
      try { await activateItem(item); }
      catch (error) { console.error(error); activationError = true; }
      finally { activating = false; }
    } else {
      const wasFocused = windowsState.delayedFocused?.hwnd === win.hwnd;
      activating = true;
      try {
        const target = itemEl ? await captureDockTarget(itemEl) : null;
        await invoke(MacUiCommand.WegToggleWindowState, {
          hwnd: win.hwnd, wasFocused, minimizeTarget: target,
        });
      } catch (error) { console.error(error); activationError = true; }
      finally { activating = false; }
    }
  }

  function onAuxClick(e: MouseEvent) {
    if (e.defaultPrevented || e.button !== 1) return;
    e.preventDefault();
    e.stopPropagation();
    if (settings?.middleClickAction === WegMiddleClickAction.OpenNewInstance) {
      launchItem(item, false);
    } else {
      const win = windows[0];
      if (win) invoke(MacUiCommand.WegCloseApp, { hwnd: win.hwnd });
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const alignX = settingsState.popupAlignX;
    const alignY = settingsState.popupAlignY;
    invoke(MacUiCommand.TriggerContextMenu, {
      menu: { ...getUserApplicationContextMenu($t, item, windows), alignX, alignY },
      forwardTo: null,
    });
  }
</script>

<div
  bind:this={itemEl}
  role="menu"
  tabindex="0"
  class="weg-item-overlay"
>
  <div
    role="menuitem"
    tabindex="0"
    class="weg-item"
    aria-busy={activating}
    aria-label={activationError ? $t("weg.activation_failed") : item.displayName}
    data-tooltip={activationError ? $t("weg.activation_failed") : windows.length ? undefined : item.displayName}
    data-tooltip-origin-y={settingsState.tooltipOrigin.y}
    data-tooltip-origin-x={settingsState.tooltipOrigin.x}
    data-tooltip-align-x={settingsState.popupAlignX}
    data-tooltip-align-y={settingsState.popupAlignY}
    onpointerenter={onEnter}
    onpointerleave={cancelHover}
    onpointerdown={onPress}
    onpointercancel={cancelHover}
    onclick={onClick}
    onauxclick={onAuxClick}
    oncontextmenu={onContextMenu}
    onkeydown={(event) => { if (event.target === event.currentTarget && !event.repeat && !event.ctrlKey && !event.altKey && !event.metaKey && (event.key === "Enter" || event.key === " ")) { event.preventDefault(); event.stopPropagation(); void onClick(); } }}
  >
    <FileIcon
      class="weg-item-icon"
      path={item.relaunch?.icon || item.path}
      umid={item.umid}
      appearance={isApp ? "app-glass" : "liquid-glass"}
    />
    {#if itemLabel}
      <div class="weg-item-title">{itemLabel}</div>
    {/if}
  </div>

  {#if activationError}<span class="activation-error" role="alert">{$t("weg.activation_failed")}</span>{/if}

  {#if notificationsCount > 0}
    <div class="weg-item-notification-badge" transition:CssHandled>
      {notificationsCount}
    </div>
  {/if}

  {#if settings?.showInstanceCounter && windows.length > 1}
    <div class="weg-item-instance-counter-badge" transition:CssHandled>
      {windows.length}
    </div>
  {/if}

  {#if !settings?.showWindowTitle}
    <div
      class="weg-item-open-sign"
      class:weg-item-open-sign-active={windows.length > 0}
      class:weg-item-open-sign-focused={isFocused}
    ></div>
  {/if}
</div>

<style>
  .activation-error { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
</style>
