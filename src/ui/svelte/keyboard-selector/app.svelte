<script lang="ts">
  import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
  import Icon from "libs/ui/svelte/components/Icon/Icon.svelte";
  import { state } from "./state.svelte";
  import { t } from "./i18n";

  $effect(() => {
    Widget.getCurrent().ready();
  });

  function onKeyboardClick(id: string, handle: string) {
    invoke(SeelenCommand.SystemSetKeyboardLayout, {
      id,
      handle,
    });
  }

  function openKeyboardSettings() {
    invoke(SeelenCommand.OpenFile, { path: "ms-settings:keyboard" });
  }

  let closing = false;
  async function handleEscape(event: KeyboardEvent) {
    if (event.key !== "Escape" || event.repeat || closing) return;
    event.preventDefault();
    closing = true;
    try { await Widget.getCurrent().hide(); }
    catch (error) { console.error("Failed to hide keyboard selector:", error); }
    finally { closing = false; }
  }
</script>

<svelte:window onkeydown={handleEscape} />

<div class={["slu-std-popover", "mac-panel", "mac-frosted-surface", "keyboard-selector"]}>
  <div class="keyboard-selector-header">{$t("title")}</div>
  <div class="keyboard-selector-body">
    {#each state.langs as lang, li}
      {#each lang.keyboardLayouts as keyboard, ki (`${li}-${keyboard.handle}-${ki}`)}
        <button
          class="layout"
          class:layout-active={keyboard.active}
          data-skin="transparent"
          aria-pressed={keyboard.active}
          onclick={() => onKeyboardClick(keyboard.id, keyboard.handle)}
        >
          <div class="layout-icon">
            <Icon iconName="FaRegKeyboard" />
          </div>
          <div class="layout-info">
            <span class="layout-lang">
              {lang.name}
            </span>
            <span class="layout-keyboard">
              {keyboard.displayName}
            </span>
          </div>
          {#if keyboard.active}<Icon class="layout-check" iconName="IoCheckmark" />{/if}
        </button>
      {/each}
    {/each}
  </div>
  <div class="keyboard-selector-footer">
    <button data-skin="transparent" onclick={openKeyboardSettings}>
      {$t("more")}
    </button>
  </div>
</div>

<style>
  :global(#root) .slu-std-popover.mac-panel.keyboard-selector {
    border-radius: 28px;
    overflow: hidden;
  }
  .keyboard-selector-header { flex-shrink: 0; }
  .keyboard-selector-body {
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--mac-track) transparent;
  }
  .layout { flex-shrink: 0; }
  .layout-info { min-width: 0; }
  .layout-lang, .layout-keyboard {
    width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
