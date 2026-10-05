<script lang="ts">
  import { onDestroy } from "svelte";
  import { DragDropProvider } from "@dnd-kit/svelte";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { createDragDropManager } from "libs/ui/dnd";
  import { globalState, type FavFolderItem, type FavPinnedItem } from "../state/mod.svelte";
  import { t } from "../i18n";
  import { pageGeometry, pageSlice } from "../launchpad";
  import { arrayMove } from "../utils";
  import SortableAppItem from "./SortableAppItem.svelte";
  import PageControls from "./PageControls.svelte";
  import PageViewport from "./PageViewport.svelte";
  import { navigateInDirection, pageShortcutStep, reconcilePageSelection, selectPreselectedOrFirst, type SelectionScope } from "../keyboard-navigation";

  let { folder, onClose }: { folder: FavFolderItem; onClose: () => void } = $props();
  const manager = createDragDropManager();
  onDestroy(() => manager.destroy());
  let dialog: HTMLDialogElement;
  let grid: HTMLUListElement;
  let selectedItemId = $state<string | null>(null);
  let folderPage = $state(0);
  let dragSnapshot = $state<FavPinnedItem[] | null>(null);
  let viewportWidth = $state(720), viewportHeight = $state(360);
  // This sheet has its own generous 64px grid; it must not inherit a compact
  // root page's capacity. Controls sit outside the measured folder viewport.
  const geometry = $derived(pageGeometry(viewportWidth, viewportHeight + 40, 64));
  const folderColumns = $derived(Math.min(5, geometry.columns));
  const folderRows = $derived(Math.min(3, geometry.rows));
  const folderPageSize = $derived(folderColumns * folderRows);
  const expandedItems = $derived(folder.itemIds.flatMap(id => {
    const item = globalState.getMenuItem(id);
    return item ? [{ id, item }] : [];
  }));
  const pagination = $derived(pageSlice(expandedItems, folderPage, folderPageSize));
  $effect(() => { folderPage = pagination.page; });
  $effect(() => {
    const next = reconcilePageSelection(selectedItemId, pagination.items.map(({ item }) => globalState.getItemId(item)));
    if (next !== selectedItemId) selectedItemId = next;
  });
  $effect(() => { if (dialog && !dialog.open) { dialog.showModal(); dialog.focus(); } });
  const scope: SelectionScope = {
    container: () => grid ?? document,
    getSelected: () => selectedItemId,
    setSelected: id => { selectedItemId = id; },
  };
  function changePage(page: number) {
    if (dragSnapshot) return;
    folderPage = Math.max(0, Math.min(pagination.count - 1, page)); selectedItemId = null;
  }
  function saveName(input: HTMLInputElement) {
    const name = input.value.trim();
    if (name !== folder.name) globalState.updateFolder(folder.itemId, { name });
  }
  function handleKeyDown(event: KeyboardEvent) {
    event.stopPropagation();
    if (dragSnapshot || event.defaultPrevented || event.isComposing || (event.target as HTMLElement).closest("input,button")) return;
    const pageStep = pageShortcutStep(event);
    if (pageStep !== null) {
      event.preventDefault();
      if (pageStep) changePage(folderPage + pageStep);
      return;
    }
    const direction = ({ ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" } as const)[event.key as "ArrowUp"];
    if (direction) { event.preventDefault(); navigateInDirection(direction, scope); }
    else if (event.key === "Enter") { event.preventDefault(); selectPreselectedOrFirst(scope)?.click(); }
  }
</script>

<dialog bind:this={dialog} tabindex="-1" class="folder-modal launchpad-folder-modal mac-frosted-surface" aria-label={folder.name || $t("folder")}
  style:--folder-columns={folderColumns} style:--folder-rows={folderRows} onkeydown={handleKeyDown} onclose={onClose}
  onclick={event => { if (event.target === dialog) dialog.close(); }}>
  <div class="folder-modal-content">
    <div class="folder-modal-heading">
      <label class="sr-only" for="launchpad-folder-name">{$t("launchpad.folder_name")}</label>
      <input id="launchpad-folder-name" type="text" class="folder-modal-name" value={folder.name} maxlength="80"
        placeholder={$t("folder")} aria-label={$t("launchpad.folder_name")}
        onblur={event => saveName(event.currentTarget)}
        onkeydown={event => { if (event.key === "Enter" && !event.isComposing) { event.preventDefault(); saveName(event.currentTarget); event.currentTarget.blur(); dialog.focus(); } event.stopPropagation(); }} />
      <button class="folder-modal-close" aria-label={$t("launchpad.close_folder")} onclick={() => dialog.close()}><Icon iconName="FiX" /></button>
    </div>
    <p class="folder-modal-caption">{$t("launchpad.app_count", { count: String(expandedItems.length) })}</p>
    <div class="folder-modal-viewport" bind:clientWidth={viewportWidth} bind:clientHeight={viewportHeight}>
    <PageViewport page={pagination.page} resetKey={`${folder.itemId}:${folderPageSize}`} disabled={!!dragSnapshot} onchange={changePage}>
    <DragDropProvider {manager} onDragStart={() => { dragSnapshot = $state.snapshot(globalState.pinnedItems); }} onDragOver={event => {
      const { source, target } = event.operation;
      if (!source || !target || source.id === target.id) return;
      const from = folder.itemIds.indexOf(String(source.id));
      const to = folder.itemIds.indexOf(String(target.id));
      if (from >= 0 && to >= 0) globalState.previewPinnedItems(globalState.pinnedItems.map(i => i.type === "folder" && i.itemId === folder.itemId ? { ...i, itemIds: arrayMove(folder.itemIds, from, to) } : i));
    }} onDragEnd={event => {
      const { source } = event.operation;
      if (event.canceled) {
        if (dragSnapshot) globalState.previewPinnedItems(dragSnapshot);
        dragSnapshot = null;
        return;
      }
      dragSnapshot = null;
      globalState.updatePinnedItems(globalState.pinnedItems);
      if (!dialog || !source?.element) return;
      const rect = source.element.getBoundingClientRect(), bounds = dialog.getBoundingClientRect();
      if (rect.right <= bounds.left || rect.left >= bounds.right || rect.bottom <= bounds.top || rect.top >= bounds.bottom) {
        globalState.moveItemToRoot(String(source.id));
        dialog.close();
      }
    }}>
      <ul bind:this={grid} role="listbox" aria-label={folder.name || $t("folder")} class="folder-modal-items">
        {#each pagination.items as { id, item }, idx (id)}<SortableAppItem {item} {idx} isInsideFolder={true} {scope} />{/each}
      </ul>
    </DragDropProvider>
    </PageViewport>
    {#if expandedItems.length === 0}<p class="launchpad-folder-empty">{$t("launchpad.empty_folder")}</p>{/if}
    </div>
    <PageControls page={pagination.page} count={pagination.count} disabled={!!dragSnapshot} onchange={changePage} />
  </div>
</dialog>
