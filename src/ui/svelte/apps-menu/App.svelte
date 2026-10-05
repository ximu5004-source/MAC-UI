<script lang="ts">
  import { tick } from "svelte";
  import { t } from "./i18n";
  import { Widget, invoke, SeelenCommand } from "@seelen-ui/lib";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import StartMenuBody from "./components/layout/StartMenuBody.svelte";
  import StartMenuFooter from "./components/layout/StartMenuFooter.svelte";
  import FolderModal from "./components/FolderModal.svelte";
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
  const geometry = $derived(pageGeometry(applicationWidth, applicationHeight));
  const openFolder = $derived(globalState.folders.find(f => f.itemId === globalState.openFolderId));
  const queryPrefix = $derived(searchState.searchQuery.trim().match(/^(apps|files|web):/i)?.[1]?.toLowerCase() || "");

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
      if (searchState.searchQuery) searchState.searchQuery = "";
      else if (globalState.view === StartView.All) { globalState.view = StartView.Favorites; globalState.page = 0; }
      else void Widget.self.hide();
    }
  }
  function handleBackgroundClick(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (document.querySelector("dialog[open]") || target.closest(".apps-menu,button,input,select,.app,.folder,dialog,a,[role=alert]")) return;
    void Widget.self.hide();
  }
  function handlePrefixChange(prefix: string) {
    const query = searchState.searchQuery.trim().replace(/^(apps|files|web):\s*/i, "");
    searchState.searchQuery = prefix ? `${prefix}: ${query}` : query;
    inputElement.focus();
  }
  const handleInputKeyDown = createInputKeyDownHandler({ onEnter: () => {
    if (queryPrefix !== "web") return false;
    const query = searchState.searchQuery.trim().slice(4).trim();
    if (query) {
      void Widget.self.hide();
      void invoke(SeelenCommand.OpenFile, { path: `https://www.google.com/search?q=${encodeURIComponent(query)}` });
    }
    return true;
  } });
  async function closeFolder() {
    const id = globalState.openFolderId;
    globalState.openFolderId = null;
    await tick();
    Array.from(document.querySelectorAll<HTMLElement>(".folder")).find(el => el.dataset.itemId === id)?.focus();
  }
</script>

<svelte:window onkeydown={handleDocKeyDown} onclick={handleBackgroundClick} onpointerdown={() => { keyboardFocus = false; }} />

<div class="apps-menu launchpad mac-frosted-surface" data-fullscreen={globalState.displayMode === StartDisplayMode.Fullscreen}
  data-searching={!!searchState.searchQuery}
  data-folder-open={!!openFolder} data-keyboard-focus={keyboardFocus} data-compact={geometry.iconSize === 48}
  style:--columns={geometry.columns} style:--launchpad-rows={geometry.rows} style:--lp-icon={`${geometry.iconSize}px`}>
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
          <button aria-pressed={globalState.view === StartView.Favorites} onclick={() => { searchState.searchQuery = ""; globalState.view = StartView.Favorites; globalState.page = 0; }}>{$t("launchpad.title")}</button>
          <button aria-pressed={globalState.view === StartView.All} onclick={() => { searchState.searchQuery = ""; globalState.view = StartView.All; globalState.page = 0; }}>{$t("all")}</button>
        </div>
      {#if searchState.searchQuery}
        <select aria-label={$t("launchpad.search_scope")} value={queryPrefix} onchange={e => handlePrefixChange(e.currentTarget.value)}>
          <option value="">{$t("query.all")}</option><option value="apps">{$t("query.apps")}</option><option value="files">{$t("query.files")}</option><option value="web">{$t("query.web")}</option>
        </select>
      {:else}
        <div class="launchpad-actions">
          <button onclick={() => { searchState.searchQuery = ""; globalState.createEmptyFolder($t("folder")); }}><Icon iconName="FiFolderPlus" />{$t("launchpad.new_folder")}</button>
          <button disabled={globalState.importing} aria-busy={globalState.importing} title={$t("launchpad.import_hint")} onclick={() => globalState.importWindowsStartMenu()}>
            <Icon iconName="FiDownload" />{globalState.importing ? $t("launchpad.importing") : $t("launchpad.import")}
          </button>
        </div>
      {/if}
    </div>
    <div class="launchpad-feedback" data-feedback-state={globalState.saveState === "error" || globalState.importFailed ? "error" : globalState.importResult || globalState.saveState === "saving" ? "status" : "hint"} aria-live="polite">
      {#if globalState.saveState === "error" || globalState.importFailed}
        <span role="alert">{$t("launchpad.save_error")}</span>
        <button onclick={() => globalState.importFailed ? globalState.importWindowsStartMenu() : globalState.retrySave()}>{$t("launchpad.retry")}</button>
      {:else if globalState.importResult}
        <span>{$t("launchpad.import_done", { apps: String(globalState.importResult.addedApps), folders: String(globalState.importResult.addedFolders) })}</span>
      {:else if globalState.saveState === "saving"}<span>{$t("launchpad.saving")}</span>
      {:else}<span>{$t("launchpad.hint")}</span>{/if}
    </div>
    <div class="launchpad-applications-viewport" bind:clientWidth={applicationWidth} bind:clientHeight={applicationHeight}>
      <StartMenuBody />
    </div>
  </section>
  <StartMenuFooter />
  {#if openFolder}<FolderModal folder={openFolder} onClose={closeFolder} />{/if}
</div>
