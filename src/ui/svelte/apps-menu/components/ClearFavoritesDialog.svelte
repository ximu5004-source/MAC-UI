<script lang="ts">
  import { t } from "../i18n";
  import { globalState } from "../state/mod.svelte";

  let { onClose }: { onClose: () => void } = $props();
  let dialog: HTMLDialogElement;
  let cancelButton: HTMLButtonElement;

  $effect(() => {
    if (dialog && !dialog.open) {
      dialog.showModal();
      cancelButton?.focus();
    }
  });

  async function confirmClear() {
    if (await globalState.clearFavorites()) {
      globalState.chooseFavorites();
      onClose();
    }
  }
</script>

<dialog bind:this={dialog} class="launchpad-clear-dialog" role="alertdialog" aria-labelledby="launchpad-clear-title" aria-describedby="launchpad-clear-description"
  oncancel={event => { if (globalState.clearingFavorites) event.preventDefault(); }} onclose={onClose}
  onclick={event => { if (event.target === dialog && !globalState.clearingFavorites) dialog.close(); }}>
  <h2 id="launchpad-clear-title">{$t("launchpad.clear_title")}</h2>
  <p id="launchpad-clear-description">{$t("launchpad.clear_description")}</p>
  {#if globalState.clearFavoritesError}<p class="launchpad-clear-error" role="alert">{$t("launchpad.clear_error")}</p>{/if}
  <div class="launchpad-clear-dialog-actions">
    <button bind:this={cancelButton} disabled={globalState.clearingFavorites} onclick={() => dialog.close()}>{$t("launchpad.cancel")}</button>
    <button class="launchpad-clear-confirm" disabled={globalState.clearingFavorites} aria-busy={globalState.clearingFavorites} onclick={confirmClear}>
      {$t(globalState.clearingFavorites ? "launchpad.clearing" : "launchpad.clear_and_choose")}
    </button>
  </div>
</dialog>
