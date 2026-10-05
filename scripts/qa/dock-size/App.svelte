<script lang="ts">
  import AppGlassIcon from "../../../libs/ui/svelte/components/Icon/AppGlassIcon.svelte";
  import { normalizeDockSize } from "../../../libs/ui/utils/dockSize";
  import "../../../src/ui/svelte/weg/styles/global.css";
  import "./preview.css";
  let size = $state(40), titles = $state(false), vertical = $state(false);
  // Production publishes the setting on :root, where derived theme sizes resolve.
  $effect(() => { document.documentElement.style.setProperty("--config-item-size", `${normalizeDockSize(size)}px`); });
  const shapes = [
    '<rect x="1" y="1" width="62" height="62" rx="14" fill="#267ed8"/><path d="M16 42V22l16 14 16-14v20" fill="none" stroke="white" stroke-width="6"/>',
    '<circle cx="32" cy="32" r="29" fill="#28aa78"/><path d="m15 33 11 11 23-24" stroke="white" stroke-width="6" fill="none"/>',
    '<path d="M4 16h24l6 7h26v31H4Z" fill="#efb62b"/>',
  ];
  const icons = shapes.map(shape => ({ src: `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">${shape}</svg>`)}`, mask: null }) as any);
</script>
<div class="qa-controls">
  <label>尺寸<input type="range" min="16" max="128" bind:value={size} /></label><output>{size} px</output>
  <label><input type="checkbox" bind:checked={titles} />窗口标题</label>
  <label><input type="checkbox" bind:checked={vertical} />垂直 Dock</label>
</div>
<div class="taskbar" class:vertical class:horizontal={!vertical} class:left={vertical} class:bottom={!vertical}
 data-has-margin="true" data-size="min-content" style:--config-item-size={`${normalizeDockSize(size)}px`}>
  <div class="weg-items-container"><div class="weg-items"><div class="weg-items-center">
    {#each [...icons, ...icons, ...icons] as state, i}
      <div class="weg-item-drag-container"><div class="weg-item-overlay"><div class="weg-item">
        <AppGlassIcon {state} class="weg-item-icon" />
        {#if titles}<div class="weg-item-title">测试应用 {i + 1} — 长标题不会挤压图标</div>{/if}
      </div></div></div>
    {/each}
  </div></div></div>
</div>
