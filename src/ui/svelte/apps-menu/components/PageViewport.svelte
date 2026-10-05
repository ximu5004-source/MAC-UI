<script lang="ts">
  import { onMount, tick, untrack, type Snippet } from "svelte";
  import { pageDirection, WheelPager, type PageLocation } from "../pageMotion";

  let { page, resetKey, disabled = false, onchange, children }: {
    page: number;
    resetKey: string;
    disabled?: boolean;
    onchange: (page: number) => void;
    children: Snippet;
  } = $props();

  let viewport: HTMLDivElement;
  let content: HTMLDivElement;
  let snapshots: HTMLDivElement;
  let previous: PageLocation | undefined;
  let animations: Animation[] = [];
  let generation = 0;
  const wheelPager = new WheelPager();

  function cancelMotion() {
    generation++;
    for (const animation of animations) animation.cancel();
    animations = [];
    snapshots?.replaceChildren();
    viewport?.removeAttribute("data-page-animating");
  }

  // State never waits for animation completion. Pending ticks and superseded animations
  // cannot restore an old page after rapid paging, a search, resize or window suspension.
  $effect.pre(() => {
    const next = { page, resetKey };
    const blocked = disabled;
    const node = content;
    if (!node || !snapshots) return;
    untrack(() => {
      const direction = pageDirection(previous, next);
      const reset = previous?.resetKey !== next.resetKey;
      previous = next;
      const startTransform = getComputedStyle(node).transform;
      cancelMotion();
      if (reset || blocked) wheelPager.reset();
      if (!direction || blocked || document.hidden || matchMedia("(prefers-reduced-motion: reduce)").matches) return;

      // A visual-only DOM snapshot has no Svelte handlers or DnD registrations.
      // Inert plus stripped identities keep it out of click, focus and selection paths.
      const outgoing = node.cloneNode(true) as HTMLDivElement;
      outgoing.inert = true;
      outgoing.setAttribute("aria-hidden", "true");
      for (const element of Array.from(outgoing.querySelectorAll("[id],[data-item-id],[tabindex]"))) {
        element.removeAttribute("id");
        element.removeAttribute("data-item-id");
        element.removeAttribute("tabindex");
      }
      outgoing.style.transform = startTransform;
      snapshots.append(outgoing);
      viewport.setAttribute("data-page-animating", "true");
      const token = generation;
      void tick().then(() => {
        if (token !== generation) return;
        if (document.hidden || matchMedia("(prefers-reduced-motion: reduce)").matches) { cancelMotion(); return; }
        const timing = { duration: 280, easing: "cubic-bezier(.22,.78,.2,1)" };
        const incoming = node.animate([
          { transform: `translate3d(${direction * 100}%,0,0)` },
          { transform: "translate3d(0,0,0)" },
        ], timing);
        const leaving = outgoing.animate([
          { transform: startTransform },
          { transform: `translate3d(${-direction * 100}%,0,0)` },
        ], { ...timing, fill: "forwards" });
        animations = [incoming, leaving];
        void Promise.all(animations.map(animation => animation.finished)).then(() => {
          if (token === generation) cancelMotion();
        }).catch(() => { /* Cancellation is the normal fast-paging path. */ });
      });
    });
  });

  onMount(() => {
    const media = matchMedia("(prefers-reduced-motion: reduce)");
    const reset = () => { cancelMotion(); wheelPager.reset(); };
    // Capture pointer intent at its current position, not during a moving target.
    const pointerStart = () => cancelMotion();
    const onWheel = (event: WheelEvent) => {
      if (disabled || (event.target as HTMLElement).closest(".launchpad-page-snapshots")) return;
      if (event.ctrlKey) { wheelPager.reset(); return; }
      // Short windows may need a folder's vertical scrollbar; do not steal that gesture.
      const folderGrid = (event.target as HTMLElement).closest<HTMLElement>(".folder-modal-items");
      if (folderGrid && Math.abs(event.deltaY) > Math.abs(event.deltaX)
        && ((event.deltaY > 0 && folderGrid.scrollTop + folderGrid.clientHeight < folderGrid.scrollHeight - 1)
          || (event.deltaY < 0 && folderGrid.scrollTop > 0))) return;
      event.preventDefault();
      event.stopPropagation();
      const direction = wheelPager.consume(event, performance.now(), viewport.clientWidth);
      if (direction) onchange(page + direction);
    };
    viewport.addEventListener("wheel", onWheel, { passive: false });
    viewport.addEventListener("pointerdown", pointerStart, { capture: true });
    document.addEventListener("visibilitychange", reset);
    window.addEventListener("blur", reset);
    window.addEventListener("resize", reset);
    media.addEventListener("change", reset);
    return () => {
      reset();
      viewport.removeEventListener("wheel", onWheel);
      viewport.removeEventListener("pointerdown", pointerStart, { capture: true });
      document.removeEventListener("visibilitychange", reset);
      window.removeEventListener("blur", reset);
      window.removeEventListener("resize", reset);
      media.removeEventListener("change", reset);
    };
  });
</script>

<div bind:this={viewport} class="launchpad-page-viewport" data-page={page}>
  <div bind:this={content} class="launchpad-page-content">{@render children()}</div>
  <div bind:this={snapshots} class="launchpad-page-snapshots" aria-hidden="true" inert></div>
</div>
