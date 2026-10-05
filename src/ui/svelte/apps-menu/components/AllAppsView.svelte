<script lang="ts">
  import { t } from "../i18n";
  import AppItem from "./AppItem.svelte";
  import { searchState, getItemKey } from "../state/search.svelte";
  import { globalState } from "../state/mod.svelte";
  import {
    navigateInDirection,
    pageShortcutStep,
    reconcilePageSelection,
    selectPreselectedOrFirst,
    type SelectionScope,
  } from "../keyboard-navigation";
  import type { StartMenuItem } from "@seelen-ui/lib/types";
  import { pageSlice } from "../launchpad";
  import PageControls from "./PageControls.svelte";
  import PageViewport from "./PageViewport.svelte";

  const pagination = $derived(pageSlice(searchState.searchedItems, globalState.page, globalState.pageSize));
  let grid: HTMLUListElement;
  $effect(() => { globalState.page = pagination.page; });
  function changePage(page: number) {
    globalState.page = Math.max(0, Math.min(pagination.count - 1, page));
    selectedItemId = null;
  }

  // Selection is fully local to this view: its own state, its own scope,
  // its own keydown handling. Nothing outside this component touches it.
  let selectedItemId = $state<string | null>(null);

  const scope: SelectionScope = {
    container: () => grid ?? document,
    getSelected: () => selectedItemId,
    setSelected: (id) => {
      selectedItemId = id;
    },
  };

  // Reset selection whenever the menu is (re)opened or the search query changes.
  // While searching, preselect the first result instead of leaving it empty.
  $effect(() => {
    globalState.version;
    searchState.searchQuery;

    const firstItem = searchState.searchQuery ? searchState.searchedItems[0] : undefined;
    selectedItemId = firstItem ? globalState.getItemId(firstItem) : null;
  });

  $effect(() => {
    const next = reconcilePageSelection(selectedItemId, pagination.items.map(item => globalState.getItemId(item)));
    if (next !== selectedItemId) selectedItemId = next;
  });

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing || document.querySelector("dialog[open]") || (event.target as HTMLElement).closest("button,select")) return;
    const pageStep = pageShortcutStep(event);
    if (pageStep !== null) {
      event.preventDefault();
      if (pageStep) changePage(pagination.page + pageStep);
      return;
    }
    switch (event.key) {
      case "Enter":
        event.preventDefault();
        selectPreselectedOrFirst(scope)?.click();
        break;
      case "ArrowUp":
        event.preventDefault();
        navigateInDirection("up", scope);
        break;
      case "ArrowDown":
        event.preventDefault();
        navigateInDirection("down", scope);
        break;
      case "ArrowLeft":
        event.preventDefault();
        navigateInDirection("left", scope);
        break;
      case "ArrowRight":
        event.preventDefault();
        navigateInDirection("right", scope);
        break;
    }
  }

</script>

<svelte:window onkeydown={handleWindowKeyDown} />
<div class="all-apps-view">
  <PageViewport page={pagination.page} resetKey={`${globalState.version}:${globalState.pageSize}:${searchState.searchQuery}`}
    onchange={changePage}>
  <ul bind:this={grid} role="listbox" aria-label={$t("applications")} class="all-apps-view-list launchpad-grid">
    {#each pagination.items as item, idx (getItemKey(item))}
      <AppItem {item} {idx} lazy {scope} />
    {/each}
  </ul>
  </PageViewport>

  {#if searchState.searchedItems.length === 0}
    <div class="all-apps-view-empty">
      {searchState.searchFilter === "web" ? $t("web_search") : $t("no_matching_items")}
    </div>
  {/if}
  <PageControls page={pagination.page} count={pagination.count} onchange={changePage} />
</div>
