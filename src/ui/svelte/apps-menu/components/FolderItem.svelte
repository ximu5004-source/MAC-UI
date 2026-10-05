<script lang="ts">
  import type { FavFolderItem } from "../state/mod.svelte";
  import { FileIcon } from "libs/ui/svelte/components/Icon";
  import { globalState } from "../state/mod.svelte";
  import { t } from "../i18n";
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { createSortable } from "@dnd-kit/svelte/sortable";
  import { getFolderContextMenu } from "./context-menu.svelte";
  import type { SelectionScope } from "../keyboard-navigation";

  interface Props {
    folder: FavFolderItem;
    idx: number;
    isActiveDropzone?: boolean;
    scope: SelectionScope;
  }

  let { folder, idx, isActiveDropzone = false, scope }: Props = $props();

  let el: HTMLLIElement;

  const sortable = createSortable({
    get id() {
      return folder.itemId;
    },
    get index() {
      return idx;
    },
    type: "folder",
  });

  const menu = $derived(getFolderContextMenu(folder, $t));

  const isSelected = $derived(scope.getSelected() === folder.itemId);
  const tabindex = $derived(isSelected || (idx === 0 && !scope.getSelected()) ? 0 : -1);

  const folderName = $derived(folder.name || $t("folder"));
  const expandedItems = $derived(
    folder.itemIds
      .map((id) => ({
        id,
        item: globalState.getMenuItem(id)!,
      }))
      .filter((app) => !!app.item),
  );

  $effect(() => {
    if (isSelected) {
      el.scrollIntoView({ block: "nearest" });
    }
  });

  function handleFolderContextMenu(event: MouseEvent) {
    event.preventDefault();
    invoke(SeelenCommand.TriggerContextMenu, {
      menu,
      forwardTo: null,
    });
  }

  function openModal() {
    if (!sortable.isDragging && !sortable.isDropping) globalState.openFolderId = folder.itemId;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      e.stopPropagation();
      openModal();
    }
  }
</script>

<li
  bind:this={el}
  {@attach sortable.attach}
  role="option"
  aria-selected={isSelected}
  aria-label={`${folderName}, ${$t("launchpad.app_count", { count: String(expandedItems.length) })}`}
  {tabindex}
  data-item-id={folder.itemId}
  class="folder"
  class:is-dragging={sortable.isDragging}
  class:is-dropping={sortable.isDropping}
  class:is-drop-target={isActiveDropzone}
  onclick={openModal}
  oncontextmenu={handleFolderContextMenu}
  onkeydown={handleKeyDown}
  onfocus={() => scope.setSelected(folder.itemId)}
>
  <div class="folder-grid">
    {#each expandedItems.slice(0, 9) as app}
      <div class="folder-preview">
        <FileIcon class="folder-preview-icon" path={app.item.path} umid={app.item.umid} appearance="plain" />
      </div>
    {/each}
    {#if !expandedItems.length}<span class="folder-empty-mark" aria-hidden="true">+</span>{/if}
  </div>
  <div class="folder-name" title={folderName}>
    {folderName}
  </div>
</li>
