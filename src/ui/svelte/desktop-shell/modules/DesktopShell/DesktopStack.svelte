<script lang="ts">
  import type { Snippet } from "svelte";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { t } from "../../i18n";
  import { clampStackPosition, clampStackSize, moveStackPosition, resizeStackRect, STACK_GRID, type ResizeEdge, type StackPoint, type StackRect, type StackSize } from "./stackLayout";

  let { title, count, collapsed, position, dimensions, bounds, active, snap, onmove, onresize, oninteraction, oncollapse, onactivate, children }: {
    title: string;
    count: number;
    collapsed: boolean;
    position: StackPoint;
    dimensions?: StackSize;
    bounds: StackSize;
    active: boolean;
    snap: boolean;
    onmove: (point: StackPoint, commit: boolean) => void;
    onresize: (point: StackPoint, size: StackSize | undefined, commit: boolean) => void;
    oninteraction: (active: boolean) => void;
    oncollapse: () => void;
    onactivate: () => void;
    children: Snippet;
  } = $props();

  let panel = $state<HTMLElement>();
  let measured = $state<StackSize>({ width: 260, height: 58 });
  let moveControls = $state(false);
  type Gesture = { pointerId: number; x: number; y: number; origin: StackRect; dimensions?: StackSize; kind: "move" | ResizeEdge; moved: boolean };
  let gesture = $state<Gesture | null>(null);
  const fittedSize = $derived(dimensions ? clampStackSize(dimensions, bounds) : undefined);
  const visiblePosition = $derived(clampStackPosition(position, bounds, measured));
  const visibleRect = $derived({ ...visiblePosition, ...measured });
  const directions = [["left", -1, 0, "TbArrowLeft"], ["up", 0, -1, "TbArrowUp"], ["down", 0, 1, "TbArrowDown"], ["right", 1, 0, "TbArrowRight"]] as const;
  const edges: ResizeEdge[] = ["n", "s", "e", "w", "ne", "nw", "se", "sw"];

  $effect(() => {
    if (!panel) return;
    const observer = new ResizeObserver(() => {
      if (panel) measured = { width: panel.offsetWidth, height: panel.offsetHeight };
    });
    observer.observe(panel);
    return () => observer.disconnect();
  });

  function startGesture(event: PointerEvent, kind: Gesture["kind"]) {
    if (event.button !== 0) return;
    event.stopPropagation();
    onactivate();
    gesture = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, origin: { ...visibleRect }, dimensions, kind, moved: false };
    if (event.currentTarget instanceof HTMLElement) event.currentTarget.setPointerCapture(event.pointerId);
  }

  function updateGesture(event: PointerEvent) {
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    const delta = { x: event.clientX - gesture.x, y: event.clientY - gesture.y };
    if (!gesture.moved && Math.abs(delta.x) + Math.abs(delta.y) < 4) return;
    gesture.moved = true;
    oninteraction(true);
    if (gesture.kind === "move") {
      onmove(moveStackPosition({ x: gesture.origin.x + delta.x, y: gesture.origin.y + delta.y }, bounds, measured, snap), false);
    } else {
      const rect = resizeStackRect(gesture.origin, delta, gesture.kind, bounds, snap);
      onresize({ x: rect.x, y: rect.y }, { width: rect.width, height: rect.height }, false);
    }
  }

  function endGesture(event: PointerEvent) {
    if (!gesture || event.pointerId !== gesture.pointerId) return;
    updateGesture(event);
    if (gesture.moved) {
      // Compute the final value directly; parent props can update on the next Svelte flush.
      const delta = { x: event.clientX - gesture.x, y: event.clientY - gesture.y };
      if (gesture.kind === "move") {
        onmove(moveStackPosition({ x: gesture.origin.x + delta.x, y: gesture.origin.y + delta.y }, bounds, measured, snap), true);
      } else {
        const rect = resizeStackRect(gesture.origin, delta, gesture.kind, bounds, snap);
        onresize({ x: rect.x, y: rect.y }, { width: rect.width, height: rect.height }, true);
      }
    }
    gesture = null;
    oninteraction(false);
  }

  function cancelGesture() {
    if (!gesture) return;
    if (gesture.kind === "move") onmove({ x: gesture.origin.x, y: gesture.origin.y }, false);
    else onresize({ x: gesture.origin.x, y: gesture.origin.y }, gesture.dimensions, false);
    gesture = null;
    oninteraction(false);
  }

  function nudge(x: number, y: number) {
    onactivate();
    onmove(moveStackPosition({ x: visiblePosition.x + x, y: visiblePosition.y + y }, bounds, measured, snap), true);
  }

  function resizeBy(x: number, y: number, edge: ResizeEdge = "se") {
    onactivate();
    const rect = resizeStackRect(visibleRect, { x, y }, edge, bounds, snap);
    onresize({ x: rect.x, y: rect.y }, { width: rect.width, height: rect.height }, true);
  }

  function handleKey(event: KeyboardEvent, edge?: ResizeEdge) {
    if (event.key === "Escape") { cancelGesture(); return; }
    const step = (snap ? STACK_GRID : 10) * (event.shiftKey ? 4 : 1);
    const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const move = moves[event.key];
    if (!move) return;
    event.preventDefault();
    event.stopPropagation();
    if (edge) resizeBy(move[0], move[1], edge);
    else nudge(move[0], move[1]);
  }
</script>

<section bind:this={panel} class="desktop-stack mac-frosted-surface" class:is-dragging={!!gesture?.moved}
  style:transform={`translate(${visiblePosition.x}px, ${visiblePosition.y}px)`}
  style:width={`${fittedSize?.width ?? Math.min(260, bounds.width)}px`}
  style:height={!collapsed && fittedSize ? `${fittedSize.height}px` : "auto"}
  style:max-height={`${dimensions ? bounds.height : Math.min(760, bounds.height)}px`}
  style:z-index={gesture ? 3 : active ? 2 : 1} aria-label={title} onfocusin={onactivate}>
  <div class="stack-header">
    <button class="stack-handle" title={$t("desktop_shell.move_hint")} aria-label={`${title} · ${$t("desktop_shell.move_hint")}`}
      onpointerdown={(event) => startGesture(event, "move")} onpointermove={updateGesture} onpointerup={endGesture}
      onpointercancel={cancelGesture} onlostpointercapture={cancelGesture} onkeydown={(event) => handleKey(event)}
      onclick={(event) => { event.stopPropagation(); onactivate(); }}>
      <Icon name="TbGripVertical" aria-hidden="true" />
      <span class="stack-title">{title}</span><span class="stack-count">{count}</span>
    </button>
    <button class="stack-action" title={$t("desktop_shell.move_controls")} aria-label={`${title} · ${$t("desktop_shell.move_controls")}`} aria-expanded={moveControls} onclick={(event) => { event.stopPropagation(); moveControls = !moveControls; onactivate(); }}>
      <Icon name="TbArrowsMove" aria-hidden="true" />
    </button>
    <button class="stack-action" aria-label={`${title} · ${$t(collapsed ? "desktop_shell.expand" : "desktop_shell.collapse")}`} aria-expanded={!collapsed} onclick={(event) => { event.stopPropagation(); oncollapse(); }}>
      <Icon name={collapsed ? "TbChevronDown" : "TbChevronUp"} aria-hidden="true" />
    </button>
  </div>
  {#if moveControls}
    <div class="stack-move-controls" role="toolbar" aria-label={$t("desktop_shell.move_controls")}>
      <div class="control-row">
        {#each directions as [direction, x, y, icon]}
          <button class="stack-action" aria-label={$t(`desktop_shell.move_${direction}`)} onclick={(event) => { event.stopPropagation(); nudge(x * STACK_GRID, y * STACK_GRID); }}><Icon name={icon} aria-hidden="true" /></button>
        {/each}
      </div>
      {#if !collapsed}
        <div class="control-row size-controls">
          <button onclick={(event) => { event.stopPropagation(); resizeBy(-STACK_GRID, 0, "e"); }}>{$t("desktop_shell.narrower")}</button>
          <button onclick={(event) => { event.stopPropagation(); resizeBy(STACK_GRID, 0, "e"); }}>{$t("desktop_shell.wider")}</button>
          <button onclick={(event) => { event.stopPropagation(); resizeBy(0, -STACK_GRID, "s"); }}>{$t("desktop_shell.shorter")}</button>
          <button onclick={(event) => { event.stopPropagation(); resizeBy(0, STACK_GRID, "s"); }}>{$t("desktop_shell.taller")}</button>
        </div>
      {/if}
    </div>
  {/if}
  {#if !collapsed}
    <div class="stack-items">{@render children()}</div>
    {#each edges as edge}
      <button class={`resize-handle resize-${edge}`} tabindex={edge === "se" ? 0 : -1}
        title={$t("desktop_shell.resize_hint")} aria-label={`${title} · ${$t(`desktop_shell.resize_${edge}`)}`}
        onpointerdown={(event) => startGesture(event, edge)} onpointermove={updateGesture} onpointerup={endGesture}
        onpointercancel={cancelGesture} onlostpointercapture={cancelGesture} onkeydown={(event) => handleKey(event, edge)}
        onclick={(event) => event.stopPropagation()}>
        {#if edge === "se"}<span aria-hidden="true"></span>{/if}
      </button>
    {/each}
  {/if}
  {#if gesture?.moved && gesture.kind !== "move"}
    <output class="size-badge">{measured.width} × {measured.height}</output>
  {/if}
</section>

<style>
  .desktop-stack { position: absolute; top: 0; left: 0; display: flex; flex-direction: column; box-sizing: border-box; padding: 7px; border: 0; border-radius: 22px; background: var(--mac-glass-surface, rgb(255 255 255 / 0.1)); box-shadow: none; backdrop-filter: var(--mac-glass-blur, blur(18px)); }
  .desktop-stack.is-dragging { box-shadow: none; }
  .stack-header { display: flex; align-items: center; gap: 2px; flex-shrink: 0; }
  .stack-handle, .stack-action, .size-controls button { display: flex; align-items: center; justify-content: center; gap: 4px; padding: 0 4px; border: 1px solid transparent; border-radius: 10px; background: transparent; color: white; font: inherit; text-shadow: 0 1px 4px rgb(0 0 0 / 0.9); cursor: pointer; }
  .stack-handle { flex: 1; min-width: 0; min-height: 42px; font-size: 12px; font-weight: 650; cursor: grab; touch-action: none; }
  .stack-handle:active { cursor: grabbing; }
  .stack-action { width: 26px; height: 30px; flex-shrink: 0; }
  button:hover { background: rgb(255 255 255 / 0.2); }
  button:focus-visible { outline: 2px solid white; outline-offset: -2px; background: rgb(255 255 255 / 0.1); }
  .stack-title { flex: 1; text-align: left; overflow-wrap: anywhere; }
  .stack-count { padding: 1px 5px; border: 1px solid rgb(255 255 255 / 0.45); border-radius: 999px; font-size: 11px; }
  .stack-items { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--desktop-cell-width, 112px)), var(--desktop-cell-width, 112px))); grid-auto-rows: min-content; align-content: start; gap: 10px 6px; justify-content: center; padding: 6px 0 16px; overflow: auto; min-height: 0; flex: 1 1 auto; scrollbar-width: thin; scrollbar-color: rgb(255 255 255 / .45) transparent; border-radius: 0 0 16px 16px; }
  .stack-move-controls { padding-bottom: 5px; flex-shrink: 0; }
  .control-row { display: flex; flex-wrap: wrap; justify-content: center; gap: 4px; }
  .size-controls button { min-height: 30px; font-size: 11px; background: rgb(255 255 255 / 0.1); }
  .resize-handle { position: absolute; z-index: 6; padding: 0; border: 0; background: transparent; touch-action: none; color: white; }
  .resize-n, .resize-s { left: 22px; right: 22px; height: 8px; cursor: ns-resize; }
  .resize-n { top: 0; } .resize-s { bottom: 0; }
  .resize-e, .resize-w { top: 22px; bottom: 22px; width: 8px; cursor: ew-resize; }
  .resize-e { right: 0; } .resize-w { left: 0; }
  .resize-ne, .resize-nw, .resize-se, .resize-sw { width: 22px; height: 22px; border-radius: 8px; }
  .resize-ne { right: 0; top: 0; cursor: nesw-resize; }
  .resize-nw { left: 0; top: 0; cursor: nwse-resize; }
  .resize-se { right: 0; bottom: 0; cursor: nwse-resize; }
  .resize-sw { left: 0; bottom: 0; cursor: nesw-resize; }
  .resize-se span { position: absolute; width: 10px; height: 10px; right: 6px; bottom: 6px; border-right: 2px solid white; border-bottom: 2px solid white; border-radius: 0 0 4px 0; opacity: 0.8; }
  .size-badge { position: absolute; pointer-events: none; right: 24px; bottom: 6px; padding: 2px 7px; border-radius: 5px; font-size: 11px; color: white; background: rgb(0 0 0 / 0.75); z-index: 7; }
</style>
