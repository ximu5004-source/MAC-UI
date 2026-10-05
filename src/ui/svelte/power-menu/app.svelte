<script lang="ts">
  import { onMount, tick } from "svelte";
  import { options, type Option } from "./options";
  import { needsPowerConfirmation } from "./actionPolicy";
  import { state as powerState } from "./state.svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Icon, MissingIcon } from "libs/ui/svelte/components/Icon";
  import { Widget } from "@seelen-ui/lib";
  import { t } from "./i18n";

  let sheet = $state<HTMLElement>();
  let cancelButton = $state<HTMLButtonElement>();
  let selected = $state<Option | null>(null);
  let busy = $state(false);
  let error = $state("");

  async function reset() {
    if (busy) return;
    selected = null; error = "";
    await tick(); cancelButton?.focus();
  }
  onMount(() => {
    void Widget.self.ready();
    void reset();
    window.addEventListener("mac-ui::power-opened", reset);
    return () => window.removeEventListener("mac-ui::power-opened", reset);
  });
  function dismiss() {
    if (!busy) void Widget.self.hide().catch(console.error);
  }
  async function execute(option: Option) {
    if (busy) return;
    busy = true; error = "";
    try {
      await option.onClick();
      await Widget.self.hide();
    } catch (cause) {
      error = String(cause);
    } finally {
      busy = false;
      if (error) { await tick(); cancelButton?.focus(); }
    }
  }
  async function choose(option: Option) {
    if (busy) return;
    error = "";
    if (!needsPowerConfirmation(option.key)) { await execute(option); return; }
    selected = option;
    await tick(); cancelButton?.focus(); // Enter must never silently confirm shutdown.
  }
  async function back() {
    if (busy) return;
    const key = selected?.key;
    selected = null; error = "";
    await tick(); sheet?.querySelector<HTMLButtonElement>('[data-action="' + key + '"]')?.focus();
  }
  function keyboard(event: KeyboardEvent) {
    if (event.isComposing || event.defaultPrevented) return;
    if (event.key === "Escape") {
      event.preventDefault(); event.stopPropagation();
      if (event.repeat) return;
      if (selected && !busy) void back(); else dismiss();
    }
    if (event.key === "Tab" && sheet) {
      const buttons = Array.from(sheet.querySelectorAll<HTMLButtonElement>("button:not(:disabled)"));
      const first = buttons[0]; const last = buttons.at(-1);
      if (!sheet.contains(document.activeElement)) { event.preventDefault(); cancelButton?.focus(); }
      else if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
  }
  const monitor = $derived(powerState.primaryMonitor);
  const placement = $derived(monitor ? "left:" + monitor.rect.left + "px;top:" + monitor.rect.top + "px;width:" + (monitor.rect.right - monitor.rect.left) / monitor.scaleFactor + "px;height:" + (monitor.rect.bottom - monitor.rect.top) / monitor.scaleFactor + "px;transform:scale(" + monitor.scaleFactor + ")" : "inset:0");
</script>

<svelte:window onkeydown={keyboard} />

<!-- Background dismissal must never happen via action bubbling. -->
<div class="power-menu-overlay mac-power" role="presentation" onclick={(event) => {
  if (event.target instanceof Element && event.target.closest(".power-sheet")) return;
  dismiss();
}}>
  <div class="power-stage" style={placement}>
    <div class="power-sheet mac-frosted-surface" role="dialog" aria-modal="true" aria-labelledby="power-title" aria-describedby="power-description" aria-busy={busy} tabindex="-1" bind:this={sheet}>
      <header class="power-heading">
        {#if powerState.user.profilePicturePath}
          <img class="power-avatar" src={convertFileSrc(powerState.user.profilePicturePath)} alt="" />
        {:else}<MissingIcon class="power-avatar" />{/if}
        <div class="power-identity"><span>{powerState.user.name}</span><span class="power-brand">MAC UI · JONA</span></div>
        <button class="power-close" aria-label={$t("cancel")} disabled={busy} onclick={dismiss}><Icon name="TbX" aria-hidden="true" /></button>
      </header>
      <h1 id="power-title">{selected ? $t(selected.key) : $t("title")}</h1>
      <p id="power-description">{selected ? $t("confirm_description." + selected.key) : $t("description")}</p>
      {#if selected}
        <div class="power-confirm-icon" class:destructive={selected.key !== "reboot"}><Icon iconName={selected.icon as any} aria-hidden="true" /></div>
      {:else}
        <div class="power-actions">
          {#each options as option}
            <button class="power-action" data-action={option.key} disabled={busy} onclick={() => choose(option)}>
              <span class="power-action-icon"><Icon iconName={option.icon as any} aria-hidden="true" /></span>
              <span class="power-action-label">{$t(option.key)}</span>
              <span class="power-action-hint">{$t("hints." + option.key)}</span>
            </button>
          {/each}
        </div>
      {/if}
      {#if error}<p class="power-error" role="alert">{$t("operation_failed")}: {error}</p>{/if}
      <footer class="power-footer">
        <span class="power-key-hint">{busy ? $t("working") : $t("escape_hint")}</span>
        <div class="power-footer-actions">
          <button class="power-cancel" bind:this={cancelButton} disabled={busy} onclick={() => selected ? back() : dismiss()}>{$t("cancel")}</button>
          {#if selected}<button class="power-confirm" class:destructive={selected.key !== "reboot"} disabled={busy} onclick={() => selected && execute(selected)}>{$t(selected.key)}</button>{/if}
        </div>
      </footer>
    </div>
  </div>
</div>
