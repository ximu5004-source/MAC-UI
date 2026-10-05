<script lang="ts">
  import DesktopShell from "./modules/DesktopShell/DesktopShell.svelte";
</script>

<DesktopShell />

<style>
  :global(html), :global(body), :global(#root) {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: hidden;
    background: transparent !important;
  }
</style>
