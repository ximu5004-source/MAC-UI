<script lang="ts">
  import type { ClassValue } from "svelte/elements";
  import type { IconState } from "./common.svelte.ts";
  import { appIconGeometry, appIconIsTile } from "./appIconGeometry";

  let { state: iconState, class: className, lazy, ...rest }: {
    state: IconState; class?: ClassValue; lazy?: boolean; [key: string]: any;
  } = $props();
  let measured = $state({ src: "", geometry: { scale: 1, x: 0, y: 0 }, tile: false });
  const current = $derived(measured.src === iconState.src ? measured : { geometry: { scale: 1, x: 0, y: 0 }, tile: false });
  const inset = $derived(current.tile ? 1 : 0.84);
  const transform = $derived(`translate(${current.geometry.x * inset}%, ${current.geometry.y * inset}%) scale(${current.geometry.scale * inset})`);
  function measure(event: Event) {
    const img = event.currentTarget as HTMLImageElement;
    try {
      const canvas = document.createElement("canvas");
      canvas.width = canvas.height = 64;
      const ctx = canvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) return;
      const scale = 64 / Math.max(img.naturalWidth, img.naturalHeight);
      const width = img.naturalWidth * scale, height = img.naturalHeight * scale;
      ctx.drawImage(img, (64 - width) / 2, (64 - height) / 2, width, height);
      const rgba = ctx.getImageData(0, 0, 64, 64).data;
      const tile = appIconIsTile(rgba, 64, 64);
      measured = { src: img.getAttribute("src") || "", geometry: appIconGeometry(rgba, 64, 64, tile), tile };
    } catch {
      // Some external icon packs disallow canvas reads; retain their complete original image.
      measured = { src: img.getAttribute("src") || "", geometry: { scale: 1, x: 0, y: 0 }, tile: false };
    }
  }
</script>

<figure {...rest} class={["slu-icon-outer", "app-glass-icon", className]} data-appearance="app-glass">
  <div class="rounded-surface" class:with-plate={!current.tile}>
  {#key iconState.src}
    <div class="artwork" style:transform={transform}>
      <img crossorigin="anonymous" src={iconState.src || ""} alt="" loading={lazy ? "lazy" : "eager"} draggable="false" onload={measure} />
      {#if iconState.mask}<span class="color-mask" style:mask-image={`url("${iconState.mask}")`}></span>{/if}
    </div>
  {/key}
  </div>
</figure>

<style>
  .app-glass-icon { position: relative; display: block; margin: 0; flex-shrink: 0; border: 0; background: none; box-shadow: none; overflow: visible; filter: none; transition: transform 160ms ease; }
  .rounded-surface { position: absolute; inset: 0; border-radius: 24%; overflow: hidden; isolation: isolate; pointer-events: none; }
  .with-plate { background: linear-gradient(145deg, #f9fcff, #e1eafa 70%, #c8d8ed); }
  .artwork { position: absolute; inset: 0; transform-origin: center; pointer-events: none; }
  .artwork img { width: 100%; height: 100%; max-width: none; object-fit: contain; display: block; }
  .color-mask { position: absolute; inset: 0; mask-repeat: no-repeat; mask-size: contain; mask-position: center; }
  .color-mask { mask-mode: luminance; background: var(--system-accent-color); mix-blend-mode: multiply; }
  @media (prefers-reduced-motion: reduce) { .app-glass-icon { transition: none; } }
</style>
