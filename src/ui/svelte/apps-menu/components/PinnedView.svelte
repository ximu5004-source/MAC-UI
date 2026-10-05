<script lang="ts">
  import { globalState, type FavPinnedItem } from "../state/mod.svelte";
  import { t } from "../i18n";
  import SortableAppItem from "./SortableAppItem.svelte";
  import FolderItem from "./FolderItem.svelte";
  import { arrayMove } from "../utils";
  import { DragDropProvider } from "@dnd-kit/svelte";
  import { debounce } from "lodash";
  import { onDestroy } from "svelte";
  import { createDragDropManager } from "libs/ui/dnd";
  import { pageSlice } from "../launchpad";
  import PageControls from "./PageControls.svelte";
  import PageViewport from "./PageViewport.svelte";
  import {
    navigateInDirection,
    pageShortcutStep,
    reconcilePageSelection,
    selectPreselectedOrFirst,
    type SelectionScope,
  } from "../keyboard-navigation";

  type UniqueIdentifier = string | number;

  // Workaround for https://github.com/clauderic/dnd-kit/issues/2112
  const manager = createDragDropManager();
  onDestroy(() => manager.destroy());

  // Selection is fully local to this view: its own state, its own scope,
  // its own keydown handling. Nothing outside this component touches it.
  let selectedItemId = $state<string | null>(null);
  let dragSnapshot = $state<FavPinnedItem[] | null>(null);
  let grid: HTMLUListElement;
  const available = $derived(globalState.pinnedItems.filter(i => i.type === "folder" || globalState.getMenuItem(i.itemId)));
  const pagination = $derived(pageSlice(available, globalState.page, globalState.pageSize));
  $effect(() => { globalState.page = pagination.page; });
  function changePage(page: number) {
    if (dragSnapshot || globalState.openFolderId) return;
    globalState.page = Math.max(0, Math.min(pagination.count - 1, page));
    selectedItemId = null;
  }

  const scope: SelectionScope = {
    container: () => grid ?? document,
    getSelected: () => selectedItemId,
    setSelected: (id) => {
      selectedItemId = id;
    },
  };

  // Reset selection whenever the menu is (re)opened
  $effect(() => {
    globalState.version;
    selectedItemId = null;
  });

  $effect(() => {
    const visibleIds = pagination.items.flatMap(pinned => {
      if (pinned.type === "folder") return [pinned.itemId];
      const item = globalState.getMenuItem(pinned.itemId);
      return item ? [globalState.getItemId(item)] : [];
    });
    const next = reconcilePageSelection(selectedItemId, visibleIds);
    if (next !== selectedItemId) selectedItemId = next;
  });

  function handleWindowKeyDown(event: KeyboardEvent) {
    if (dragSnapshot || event.defaultPrevented || event.isComposing || document.querySelector("dialog[open]") || (event.target as HTMLElement).closest("button,select")) return;
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

  // Position-based sorting threshold (20% of item width)
  const POSITION_THRESHOLD = 0.2;
  let activeDropzoneId: UniqueIdentifier | null = $state(null);

  // Debounced folder creation activation
  const FOLDER_CREATION_DELAY_MS = 200;
  const activateDropzone = debounce((targetId: UniqueIdentifier) => {
    activeDropzoneId = targetId;
  }, FOLDER_CREATION_DELAY_MS);

  function cancelDropzone() {
    activateDropzone.cancel();
    activeDropzoneId = null;
  }
  onDestroy(() => activateDropzone.cancel());
</script>

<svelte:window onkeydown={handleWindowKeyDown} />
<div class="pinned-view">
  <PageViewport page={pagination.page} resetKey={`${globalState.version}:${globalState.pageSize}:${globalState.openFolderId ?? ""}`}
    disabled={!!dragSnapshot || !!globalState.openFolderId} onchange={changePage}>
  <ul bind:this={grid} class="pinned-view-list launchpad-grid" role="listbox" aria-label={$t("applications")}>
    <DragDropProvider
      {manager}
      onDragStart={() => { dragSnapshot = $state.snapshot(globalState.pinnedItems); }}
      onDragMove={(event) => {
        const { source, target } = event.operation;

        // No collision - cancel dropzone
        if (!source || !target || source.id === target.id) {
          cancelDropzone();
          return;
        }

        const sourceIndex = globalState.pinnedItems.findIndex((item) => item.itemId === source.id);
        const targetIndex = globalState.pinnedItems.findIndex((item) => item.itemId === target.id);

        if (sourceIndex === -1 || targetIndex === -1) {
          return;
        }

        const sourceElement = source.element;
        const targetElement = target.element;
        if (!sourceElement || !targetElement) {
          return;
        }

        const targetRect = targetElement.getBoundingClientRect();
        const sourceRect = sourceElement.getBoundingClientRect();

        // Calculate relative position (0 = left edge, 1 = right edge)
        const sourceCenterX = sourceRect.left + sourceRect.width / 2;
        const targetLeft = targetRect.left;
        const targetWidth = targetRect.width;
        const relativePosition = (sourceCenterX - targetLeft) / targetWidth;

        let shouldSort = false;
        if (sourceIndex > targetIndex) {
          shouldSort = relativePosition < POSITION_THRESHOLD;
        } else if (sourceIndex < targetIndex) {
          shouldSort = relativePosition > 1 - POSITION_THRESHOLD;
        }

        if (shouldSort && sourceIndex !== targetIndex) {
          cancelDropzone();
          globalState.previewPinnedItems(arrayMove(globalState.pinnedItems, sourceIndex, targetIndex));
          return;
        }

        if (source.type === "folder" && target.type === "app") {
          cancelDropzone();
          return;
        }
        activateDropzone(target.id);
      }}
      onDragOver={(event) => {
        event.preventDefault();
      }}
      onDragEnd={(event) => {
        const { source, target } = event.operation;
        if (event.canceled) {
          if (dragSnapshot) globalState.previewPinnedItems(dragSnapshot);
          dragSnapshot = null;
          cancelDropzone();
          return;
        }
        // A fast release on the center is still an intentional folder drop,
        // even if the hover highlight's debounce has not elapsed yet.
        activateDropzone.flush();
        activateDropzone.cancel();
        dragSnapshot = null;

        // Create folder if dropzone was active
        if (activeDropzoneId && source && target && target.id === activeDropzoneId) {
          const sourceId = source.id.toString();
          const targetId = activeDropzoneId.toString();

          // Find source and target items
          const sourceItem = globalState.pinnedItems.find((item) => item.itemId === sourceId);
          const targetIndex = globalState.pinnedItems.findIndex((item) => item.itemId === targetId);
          const targetItem = globalState.pinnedItems[targetIndex];

          if (!targetItem || !sourceItem) {
            globalState.updatePinnedItems(globalState.pinnedItems);
            cancelDropzone();
            return;
          }

          // Case 1: Source is folder + Target is folder - merge folders
          if (sourceItem.type === "folder" && targetItem.type === "folder") {
            globalState.mergeFolders(sourceItem.itemId, targetItem.itemId);
          }
          // Case 2: Source is app + Target is folder - add app to existing folder
          else if (sourceItem.type === "app" && targetItem.type === "folder") {
            globalState.addItemToFolder(targetItem.itemId, sourceId);
          }
          // Case 3: Source is app + Target is app - create new folder with both items
          else if (sourceItem.type === "app" && targetItem.type === "app") {
            // Don't create folder if dragging onto self
            if (sourceId !== targetId) {
              globalState.createFolder(sourceId, targetId, targetIndex);
            }
          }
        }
        globalState.updatePinnedItems(globalState.pinnedItems);
        cancelDropzone();
      }}
    >
      {#each pagination.items as pinnedItem, idx (pinnedItem.itemId)}
        {#if pinnedItem.type === "app"}
          {@const item = globalState.getMenuItem(pinnedItem.itemId)}
          {#if item}
            <SortableAppItem
              {item}
              {idx}
              isActiveDropzone={activeDropzoneId === pinnedItem.itemId}
              {scope}
            />
          {/if}
        {:else if pinnedItem.type === "folder"}
          {@const folder = pinnedItem}
          <FolderItem
            {folder}
            {idx}
            isActiveDropzone={activeDropzoneId === pinnedItem.itemId}
            {scope}
          />
        {/if}
      {/each}
    </DragDropProvider>
  </ul>
  </PageViewport>

  {#if globalState.pinnedItems.length === 0}
    <div class="pinned-view-empty">
      <p>{$t("welcome_message")}</p>
      <p>{$t("welcome_message2")}</p>
    </div>
  {/if}
  <PageControls page={pagination.page} count={pagination.count} disabled={!!dragSnapshot} onchange={changePage} />
</div>
