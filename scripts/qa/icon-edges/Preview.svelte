<script lang="ts">
  import AppGlassIcon from "../../../libs/ui/svelte/components/Icon/AppGlassIcon.svelte";
  import SluIconRenderer from "../../../libs/ui/svelte/components/Icon/SluIconRenderer.svelte";
  const art = (body: string) => `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">${body}</svg>`)}`;
  const icons = [
    { name: "圆形", src: art('<circle cx="32" cy="32" r="29" fill="#1981de"/><circle cx="32" cy="32" r="13" fill="#fff"/>') },
    { name: "方形", src: art('<rect x="1" y="1" width="62" height="62" rx="12" fill="#246abd"/><path d="M12 40 32 15 52 40" fill="none" stroke="white" stroke-width="7"/>') },
    { name: "长条", src: art('<rect x="4" y="22" width="56" height="20" rx="8" fill="#d65827"/>') },
  ].map(icon => ({ ...icon, mask: null, isAproximatelySquare: false }));
  let size = $state(64);
  let dark = $state(false);
</script>

<main class:dark>
  <h1>图标边缘回归</h1>
  <p>真实图标组件；悬停、键盘聚焦时不应出现光晕，图形不得拉伸或裁切。</p>
  <label>大小 {size}px <input aria-label="图标大小" type="range" min="16" max="128" bind:value={size} /></label>
  <button onclick={() => dark = !dark}>切换深浅背景</button>
  <div class="taskbar" style={`--config-item-size:${size}px`}>
    {#each icons as icon}
      <button class="weg-item" aria-label={icon.name}>
        <AppGlassIcon state={icon} style={`width:${size}px;height:${size}px`} />
      </button>
    {/each}
    {#if icons[2]}
      <button class="weg-item" aria-label="文件图标">
        <SluIconRenderer state={icons[2]} data-appearance="liquid-glass" style={`width:${size}px;height:${size}px`} />
      </button>
    {/if}
  </div>
</main>

<style>
  :global(body) { margin: 0; font: 16px system-ui; }
  main { min-height: 650px; padding: 32px; color: #14191f; background: repeating-conic-gradient(#fff 0 25%, #d8e2e9 0 50%) 0/32px 32px; }
  main.dark { color: white; background: repeating-conic-gradient(#16222c 0 25%, #35434f 0 50%) 0/32px 32px; }
  .taskbar { display: flex; align-items: center; gap: 36px; padding: 40px; margin-top: 36px; box-shadow: none; }
  button { color: inherit; cursor: pointer; }
  .weg-item { display: flex; border: 0; background: transparent; }
  .weg-item:focus-visible { outline: 2px solid currentColor; outline-offset: 10px; }
</style>
