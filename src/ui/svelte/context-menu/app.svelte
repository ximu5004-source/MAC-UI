<script lang="ts">
  import { Widget } from "@macui/lib";
  import { state } from "./state.svelte";
  import MenuItem from "./MenuItem.svelte";
  import Submenu from "./Submenu.svelte";

  $effect(() => {
    Widget.getCurrent().ready();
  });
</script>

<div class="macui-std-popover context-menu mac-frosted-surface">
  {#each state.data?.items || [] as item}
    {#if item.type === "Separator"}
      <hr />
    {:else if item.type === "Item"}
      <MenuItem {item} />
    {:else if item.type === "Submenu"}
      <Submenu {item} />
    {/if}
  {/each}
</div>
