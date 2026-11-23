<script lang="ts">
  import type { ToolbarItem } from "@macui/lib/types";
  import { createSortable } from "@dnd-kit/svelte/sortable";
  import { RestrictToHorizontalAxis } from "@dnd-kit/abstract/modifiers";
  import { toolbarState } from "../state/items.svelte.ts";
  import Item from "./Item.svelte";

  interface Props {
    module: ToolbarItem;
    index: number;
    pluginId?: string;
  }

  let { module, index, pluginId }: Props = $props();

  const sortable = createSortable({
    get id() {
      return module.id;
    },
    get index() {
      return index;
    },
    get disabled() {
      // Lock sorting, not the module's button. Pointer activation is gated by Toolbar.
      // Marking the draggable disabled makes dnd-kit declare aria-disabled on the control.
      return { draggable: false, droppable: toolbarState.isReorderDisabled };
    },
    modifiers: [RestrictToHorizontalAxis],
  });
</script>

<Item {module} {sortable} {pluginId} />
