<script lang="ts">
  import { tick } from "svelte";
  import { t } from "./i18n";
  import { Widget } from "@macui/lib";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import StartMenuBody from "./components/layout/StartMenuBody.svelte";
  import StartMenuFooter from "./components/layout/StartMenuFooter.svelte";
  import FolderModal from "./components/FolderModal.svelte";
  import ClearFavoritesDialog from "./components/ClearFavoritesDialog.svelte";
  import { globalState } from "./state/mod.svelte";
  import { searchState } from "./state/search.svelte";
  import { StartDisplayMode, StartView } from "./constants";
  import { createInputKeyDownHandler } from "./keyboard-navigation";
  import { pageGeometry } from "./launchpad";

  let inputElement: HTMLInputElement;
  // Measure only the application viewport; fixed search and controls must
  // never squeeze large native icons or their two-line labels.
  let applicationWidth = $state(960), applicationHeight = $state(600);
  let keyboardFocus = $state(false);
  let clearRequested = $state(false);
  let clearTrigger: HTMLButtonElement;
  const geometry = $derived(pageGeometry(applicationWidth, applicationHeight));
  const openFolder = $derived(globalState.folders.find(f => f.itemId === globalState.openFolderId));

  $effect(() => { globalState.columns = geometry.columns; globalState.pageSize = geometry.pageSize; });
  $effect(() => {
    if (globalState.version) {
      searchState.searchQuery = "";
      globalState.openFolderId = null;
      inputElement?.focus();
    }
  });
  $effect(() => {
    searchState.searchQuery;
    globalState.page = 0;
    if (searchState.searchQuery) globalState.view = StartView.All;
  });
  $effect(() => { void Widget.self.ready({ show: false }); });

  function handleDocKeyDown(event: KeyboardEvent) {
    if (event.key === "Tab") keyboardFocus = true;
    if (event.isComposing || event.defaultPrevented || document.querySelector("dialog[open]")) return;
    if (event.key === "Escape") {
      event.preventDefault();
      if (globalState.choosingFavorites) {
        globalState.choosingFavorites = false;
        searchState.searchQuery = "";
        globalState.view = StartView.Favorites;
      } else if (searchState.searchQuery) searchState.searchQuery = "";
      else if (globalState.view === StartView.All) { globalState.view = StartView.Favorites; globalState.page = 0; }
      else void Widget.self.hide();
    }
  }
  function handleBackgroundClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (document.querySelector("dialog[open]") || target.closest("button,input,select,.app,.folder,dialog,a,[role=alert],.launchpad-header,.launchpad-toolbar,.launchpad-feedback,.apps-menu-footer")) return;
    void Widget.self.hide();
  }
  const handleInputKeyDown = createInputKeyDownHandler();
  function selectView(view: StartView) {
    if (globalState.clearingFavorites) return;
    searchState.searchQuery = "";
    globalState.choosingFavorites = false;
    globalState.view = view;
    globalState.page = 0;
  }
  async function closeFolder() {
    const id = globalState.openFolderId;
    globalState.openFolderId = null;
    await tick();
    Array.from(document.querySelectorAll<HTMLElement>(".folder")).find(el => el.dataset.itemId === id)?.focus();
  }
  async function closeClearDialog() {
    clearRequested = false;
    await tick();
    if (clearTrigger && !clearTrigger.disabled) clearTrigger.focus();
    else inputElement?.focus();
  }
</script>

<svelte:window onkeydown={handleDocKeyDown} onclick={handleBackgroundClick} onpointerdown={() => { keyboardFocus = false; }} />

<div class="apps-menu launchpad mac-frosted-surface" data-fullscreen={globalState.displayMode === StartDisplayMode.Fullscreen}
  data-searching={!!searchState.searchQuery}
  data-folder-open={!!openFolder} data-keyboard-focus={keyboardFocus} data-compact={geometry.iconSize === 48}
  style:--columns={geometry.columns} style:--launchpad-rows={geometry.rows} style:--lp-icon={`${geometry.iconSize}px`}>
  <div class="launchpad-content">
  <header class="launchpad-header">
    <h1 class="sr-only">{$t("launchpad.title")}</h1>
    <div class="launchpad-search" role="search">
      <Icon iconName="FiSearch" aria-hidden="true" />
      <label class="sr-only" for="launchpad-search">{$t("launchpad.search")}</label>
      <input id="launchpad-search" bind:this={inputElement} bind:value={searchState.searchQuery}
        type="search" placeholder={$t("launchpad.search")} autocomplete="off" spellcheck="false" onkeydown={handleInputKeyDown} />
      {#if searchState.searchQuery}
        <button aria-label={$t("launchpad.clear_search")} onclick={() => { searchState.searchQuery = ""; inputElement.focus(); }}><Icon iconName="FiX" /></button>
      {/if}
    </div>
    <button class="launchpad-close" aria-label={$t("launchpad.close")} title={$t("launchpad.close")} onclick={() => Widget.self.hide()}><Icon iconName="FiX" /></button>
  </header>

  <section class="launchpad-applications" aria-label={$t("applications")} aria-hidden={!!openFolder} inert={!!openFolder}>
    <div class="launchpad-toolbar">
        <div class="launchpad-view-switch" aria-label={$t("launchpad.view")}>
          <button aria-pressed={globalState.view === StartView.Favorites} onclick={() => selectView(StartView.Favorites)}>{$t("launchpad.favorites")}</button>
          <button aria-pressed={globalState.view === StartView.All} onclick={() => selectView(StartView.All)}>{$t("all")}</button>
        </div>
      <div class="launchpad-actions">
        {#if globalState.canUndoClear}
          <button disabled={globalState.clearingFavorites} onclick={() => globalState.undoClearFavorites()}><Icon iconName="FiRotateCcw" />{$t("launchpad.undo_clear")}</button>
        {/if}
        <button bind:this={clearTrigger} class="launchpad-clear" disabled={!globalState.canEditFavorites || globalState.clearingFavorites || !globalState.pinnedItems.length}
          title={$t("launchpad.clear_hint")} onclick={() => { clearRequested = true; }}><Icon iconName="FiTrash2" />{$t("launchpad.clear_favorites")}</button>
        {#if globalState.choosingFavorites}
          <button onclick={() => selectView(StartView.Favorites)}><Icon iconName="FiCheck" />{$t("launchpad.done")}</button>
        {:else}
          <button disabled={!globalState.canEditFavorites} onclick={() => { searchState.searchQuery = ""; globalState.chooseFavorites(); }}><Icon iconName="FiPlus" />{$t("launchpad.add_favorites")}</button>
          {#if globalState.view === StartView.Favorites}
          <button disabled={!globalState.canEditFavorites} onclick={() => { searchState.searchQuery = ""; globalState.createEmptyFolder($t("folder")); }}><Icon iconName="FiFolderPlus" />{$t("launchpad.new_folder")}</button>
          {:else}
          <button disabled={globalState.refreshing} aria-busy={globalState.refreshing} title={$t("launchpad.all_apps_hint")} onclick={() => globalState.refreshApplications()}>
            <Icon iconName="FiRefreshCw" />{$t("launchpad.refresh")}
          </button>
          {/if}
        {/if}
      </div>
    </div>
    <div class="launchpad-feedback" data-feedback-state={globalState.layoutLoadError || globalState.saveState === "error" || globalState.refreshFailed ? "error" : globalState.saveState === "saving" ? "status" : "hint"} aria-live="polite">
      {#if globalState.layoutLoadError}
        <span role="alert">{$t("launchpad.load_error")}</span>
        <button disabled={globalState.layoutLoading} onclick={() => globalState.retrySave()}>{$t("launchpad.retry")}</button>
      {:else if globalState.saveState === "error" || globalState.refreshFailed}
        <span role="alert">{$t("launchpad.save_error")}</span>
        <button onclick={() => globalState.refreshFailed ? globalState.refreshApplications() : globalState.retrySave()}>{$t("launchpad.retry")}</button>
      {:else if globalState.saveState === "saving"}<span>{$t("launchpad.saving")}</span>
      {:else if globalState.choosingFavorites}<span>{$t("launchpad.choose_hint")}</span>
      {:else if globalState.view === StartView.All}<span>{$t("launchpad.all_apps_hint")}</span>
      {:else if globalState.retainedLegacyLayout}<span>{$t("launchpad.migrated_hint")}</span>
      {:else}<span>{$t("launchpad.hint")}</span>{/if}
    </div>
    <div class="launchpad-applications-viewport" bind:clientWidth={applicationWidth} bind:clientHeight={applicationHeight}>
      <StartMenuBody />
    </div>
  </section>
  <StartMenuFooter />
  </div>
  {#if openFolder}<FolderModal folder={openFolder} onClose={closeFolder} />{/if}
  {#if clearRequested}<ClearFavoritesDialog onClose={closeClearDialog} />{/if}
</div>
