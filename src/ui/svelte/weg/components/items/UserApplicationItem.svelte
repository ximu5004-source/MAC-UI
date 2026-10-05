<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { window as TauriWindow } from "@seelen-ui/lib/tauri";
  import { WegMiddleClickAction, type UserAppWindow } from "@seelen-ui/lib/types";
  import { FileIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import { t } from "../../i18n/index.ts";
  import type { AppOrFileWegItem } from "../../types.ts";
  import { settingsState } from "../../state/settings.svelte.ts";
  import { windowsState, focused } from "../../state/windows.svelte.ts";
  import { notifications } from "../../state/getters.svelte.ts";
  import { activateItem, getUserApplicationContextMenu, launchItem } from "../../appMenu.ts";
  import { triggerPreviewWidget } from "../../previewWidget.ts";
  import { CssHandled } from "libs/ui/svelte/utils/animations.ts";
  import { isDockPrimaryClick } from "../../activation.ts";
  import { minimizeTarget } from "../../minimizeTarget.ts";

  interface Props {
    item: AppOrFileWegItem;
    windows: UserAppWindow[];
  }

  let { item, windows }: Props = $props();

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
    const path = item.path;
    shortcutIsApp = false;
    if (!/\.lnk$/i.test(path)) return;
    let disposed = false;
    invoke(SeelenCommand.IsApplicationFile, { path }).then((value) => {
      if (!disposed) shortcutIsApp = value;
    }).catch(console.error);
    return () => { disposed = true; };
  });

  async function onClick(event?: MouseEvent) {
    if (event && !isDockPrimaryClick(event)) return;
    if (activating) return;
    activationError = false;
    if (windows.length > 1) {
      triggerPreviewWidget(itemEl!, windows);
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
        let target = null;
        if (wasFocused && itemEl && !globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches) {
          try {
            let timeout: ReturnType<typeof setTimeout> | undefined;
            const origin = await Promise.race([
              TauriWindow.getCurrentWindow().innerPosition(),
              new Promise<never>((_, reject) => {
                timeout = setTimeout(() => reject(new Error("Dock geometry timeout")), 60);
              }),
            ]).finally(() => clearTimeout(timeout));
            const icon = itemEl.querySelector(".weg-item-icon") ?? itemEl;
            target = minimizeTarget(icon.getBoundingClientRect(), origin, globalThis.devicePixelRatio);
          } catch { /* Geometry failure must not prevent native minimize. */ }
        }
        await invoke(SeelenCommand.WegToggleWindowState, {
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
      if (win) invoke(SeelenCommand.WegCloseApp, { hwnd: win.hwnd });
    }
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    const alignX = settingsState.popupAlignX;
    const alignY = settingsState.popupAlignY;
    invoke(SeelenCommand.TriggerContextMenu, {
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
    data-tooltip={activationError ? $t("weg.activation_failed") : item.displayName}
    data-tooltip-origin-y={settingsState.tooltipOrigin.y}
    data-tooltip-origin-x={settingsState.tooltipOrigin.x}
    data-tooltip-align-x={settingsState.popupAlignX}
    data-tooltip-align-y={settingsState.popupAlignY}
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
