<script lang="ts">
  import { globalState } from "./state.svelte";
  import { Widget } from "@macui/lib";
  import MainView from "./components/MainView.svelte";
  import DeviceView from "./components/DeviceView.svelte";
  import NowPlayingView from "./components/NowPlayingView.svelte";
  import { onMount } from "svelte";

  function handleBack() {
    globalState.view = "main";
    globalState.selectedDeviceId = null;
  }

  $effect(() => {
    Widget.getCurrent().ready();
  });

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.repeat) return;
      event.preventDefault();
      void Widget.getCurrent().hide().catch(console.error);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  });
</script>

<div class="macui-std-popover mac-panel mac-frosted-surface media-popup">
  {#if globalState.view === "main"}
    <MainView />
  {:else if globalState.view === "now-playing"}
    <NowPlayingView />
  {:else}
    <DeviceView onBack={handleBack} />
  {/if}
</div>
