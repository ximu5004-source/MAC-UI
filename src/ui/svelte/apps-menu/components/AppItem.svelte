<script lang="ts">
  import type { StartMenuItem } from "@macui/lib/types";
  import { Widget, invoke, MacUiCommand } from "@macui/lib";
  import { FileIcon, Icon } from "libs/ui/svelte/components/Icon";
  import { globalState } from "../state/mod.svelte";
  import { t } from "../i18n";
  import type { createSortable } from "@dnd-kit/svelte/sortable";
  import { getItemContextMenu } from "./context-menu.svelte";
  import type { SelectionScope } from "../keyboard-navigation";

  interface Props {
    item: StartMenuItem;
    idx: number;
    isActiveDropzone?: boolean;
    lazy?: boolean;
    pinSelection?: boolean;
    sortable?: ReturnType<typeof createSortable> | null;
    scope: SelectionScope;
  }

  let {
    item,
    idx,
    isActiveDropzone = false,
    lazy = false,
    pinSelection = false,
    sortable = null,
    scope,
  }: Props = $props();

  let el: HTMLLIElement;

  const itemId = $derived(item.umid || item.path.toLowerCase());
  const isSelected = $derived(scope.getSelected() === itemId);
  const tabindex = $derived(isSelected || (idx === 0 && !scope.getSelected()) ? 0 : -1);

  const menu = $derived(getItemContextMenu(item, $t));
  const noopAttach = () => {};

  $effect(() => {
    if (isSelected) {
      el.scrollIntoView({ block: "nearest" });
    }
  });

  function handleClick() {
    if (sortable?.isDragging || sortable?.isDropping) return;
    if (pinSelection) { globalState.togglePin(item); return; }
    Widget.self.hide();
    const program = item.umid ? `shell:AppsFolder\\${item.umid}` : item.path;
    invoke(MacUiCommand.OpenFile, { path: program });
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    invoke(MacUiCommand.TriggerContextMenu, {
      menu,
      forwardTo: null,
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      e.stopPropagation();
      handleClick();
    }
  }
</script>

<li
  bind:this={el}
  {@attach sortable?.attach ?? noopAttach}
  role="option"
  aria-selected={pinSelection ? globalState.isPinned(item) : isSelected}
  aria-label={pinSelection ? `${item.display_name} · ${globalState.isPinned(item) ? $t("unpin") : $t("pin")}` : item.display_name}
  {tabindex}
  data-item-id={itemId}
  class="app"
  class:choosing-favorite={pinSelection}
  class:is-dragging={sortable?.isDragging}
  class:is-dropping={sortable?.isDropping}
  class:is-drop-target={isActiveDropzone}
  onclick={handleClick}
  oncontextmenu={handleContextMenu}
  onkeydown={handleKeyDown}
  onfocus={() => scope.setSelected(itemId)}
>
  {#if pinSelection}
    <span class="launchpad-pin-state" data-pinned={globalState.isPinned(item)} aria-hidden="true"><Icon iconName={globalState.isPinned(item) ? "FiCheck" : "FiPlus"} /></span>
  {/if}
  <FileIcon
    class="app-icon"
    path={item.path}
    umid={item.umid}
    {lazy}
    appearance="plain"
  />
  <div class="app-name" title={item.display_name}>
    {item.display_name}
  </div>
</li>
