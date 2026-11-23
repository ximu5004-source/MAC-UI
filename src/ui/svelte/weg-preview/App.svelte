<script lang="ts">
  import { event as events } from "@macui/lib/tauri";
  import { containsPoint } from "./hoverBounds.ts";
  import { invoke, MacUiCommand, MacUiEvent, subscribe, Widget } from "@macui/lib";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon/index.ts";
  import { previewState } from "./state.svelte.ts";

  $effect(() => {
    Widget.getCurrent().ready();
  });

  $effect(() => {
    if (previewState.currentInteractables.length === 0) {
      Widget.self.hide();
    }
  });

  let overPreview = $state(false);

  let mouse = $state<[number, number] | null>(null);
  const outside = $derived(!!previewState.hoverAnchor && !!mouse && !overPreview &&
    !containsPoint(previewState.hoverAnchor, mouse[0], mouse[1]));

  $effect(() => {
    const anchor = previewState.hoverAnchor;
    if (!anchor) return;
    let active = true;
    let moved = false;
    mouse = null;
    const stop = subscribe(MacUiEvent.GlobalMouseMove, ({ payload }) => {
      if (active) { moved = true; mouse = payload; }
    });
    void stop.then(() => invoke(MacUiCommand.GetMousePosition)).then((point) => {
      if (active && !moved) mouse = point;
    }).catch(console.error);
    const dismiss = events.listen("macui:dismiss-dock-hover", () => {
      if (active) void Widget.self.hide();
    });
    return () => {
      active = false;
      void stop.then((unlisten) => unlisten());
      void dismiss.then((unlisten) => unlisten());
    };
  });

  $effect(() => {
    if (!outside) return;
    const timer = setTimeout(() => { if (outside) void Widget.self.hide(); }, 240);
    return () => clearTimeout(timer);
  });

  function onClickPreview(hwnd: number) {
    const target = globalThis.matchMedia("(prefers-reduced-motion: reduce)").matches
      ? null : previewState.transitionTarget;
    void Widget.self.hide().then(() => invoke(MacUiCommand.WegToggleWindowState, {
      hwnd, wasFocused: false, minimizeTarget: target,
    })).catch(console.error);
  }

  function onClosePreview(e: MouseEvent, hwnd: number) {
    e.stopPropagation();
    invoke(MacUiCommand.WegCloseApp, { hwnd });
  }

  function onAuxClickPreview(e: MouseEvent, hwnd: number) {
    if (e.button === 1) {
      invoke(MacUiCommand.WegCloseApp, { hwnd });
    }
  }
</script>

<div class="weg-item-preview-container macui-std-popover" role="group"
  onpointerenter={() => overPreview = true} onpointerleave={() => overPreview = false}>
  <div class="weg-item-preview-list">
    {#each previewState.currentInteractables as win (win.hwnd)}
      {@const preview = previewState.previews.value[win.hwnd]}
      <div
        role="button"
        tabindex="0"
        class="weg-item-preview"
        onclick={() => onClickPreview(win.hwnd)}
        onauxclick={(e) => onAuxClickPreview(e, win.hwnd)}
        onkeydown={(event) => {
          if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) {
            event.preventDefault();
            onClickPreview(win.hwnd);
          }
        }}
      >
        <div class="weg-item-preview-topbar">
          <div class="weg-item-preview-title">{win.title}</div>
          <button data-skin="transparent" onclick={(e) => onClosePreview(e, win.hwnd)}>
            <Icon iconName="IoClose" />
          </button>
        </div>
        <div class="weg-item-preview-image-container">
          {#if preview}
            <img
              class="weg-item-preview-image"
              src="data:image/webp;base64,{preview.data}"
              alt={win.title}
            />
          {:else}
            <MissingIcon class="weg-item-no-preview" />
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
