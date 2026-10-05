export type RegionRect = [number, number, number, number];
export interface RegionSnapshot { viewport: [number, number]; rects: RegionRect[] }

export function clipRegion(rect: RegionRect, bounds: RegionRect): RegionRect | null {
  const result: RegionRect = [Math.max(rect[0], bounds[0]), Math.max(rect[1], bounds[1]),
    Math.min(rect[2], bounds[2]), Math.min(rect[3], bounds[3])];
  return result.every(Number.isFinite) && result[2] > result[0] && result[3] > result[1] ? result : null;
}

/** Update the HWND's union region, not a fullscreen CSS pointer-events overlay. */
export function trackDesktopRegions(send: (snapshot: RegionSnapshot) => Promise<void>, onError: (error: unknown) => void) {
  const selector = ".desktop-stack, .desktop-controls, .desktop-status, .desktop-error, .desktop-grid > .desktop-item";
  let frame = 0;
  let disposed = false;
  let lastKey = "";
  let pending: RegionSnapshot | null = null;
  let writing: Promise<void> | null = null;
  let captured: RegionRect[] = [];
  const watched = new Set<Element>();

  function collect(): RegionSnapshot {
    const viewport: [number, number] = [window.innerWidth, window.innerHeight];
    const screen: RegionRect = [0, 0, ...viewport];
    const rects: RegionRect[] = [];
    const nodes = document.querySelectorAll<HTMLElement>(selector);
    for (const node of Array.from(nodes)) {
      if (!watched.has(node)) { watched.add(node); resize.observe(node); }
      const r = node.getBoundingClientRect();
      if (!r.width || !r.height) continue;
      // Handles stay inside the painted surface. Do not retain a rectangular
      // shadow/input gutter around rounded glass. Name-mode icons stay clipped
      // to their scrolling grid so off-screen icons cannot claim other windows.
      let bounds = screen;
      if (node.parentElement?.classList.contains("desktop-grid")) {
        const grid = node.parentElement.getBoundingClientRect();
        bounds = clipRegion([grid.left, grid.top, grid.right, grid.bottom], screen) ?? [0, 0, 0, 0];
      }
      const clipped = clipRegion([r.left, r.top, r.right, r.bottom], bounds);
      if (clipped) rects.push(clipped);
      // Expanded selected filenames and inline rename inputs belong to the icon.
      if (node.matches(".desktop-item.is-selected")) {
        for (const label of Array.from(node.querySelectorAll(".desktop-item-name, input"))) {
          const l = label.getBoundingClientRect();
          const extra = clipRegion([l.left, l.top, l.right, l.bottom], bounds);
          if (extra) rects.push(extra);
        }
      }
    }
    for (const node of watched) {
      if (!node.isConnected) { resize.unobserve(node); watched.delete(node); }
    }
    return { viewport, rects: [...rects, ...captured].slice(0, 512) };
  }

  function pump(): Promise<void> {
    if (writing) return writing;
    writing = (async () => {
      while (pending && !disposed) {
        const next = pending; pending = null;
        const key = JSON.stringify(next);
        if (key === lastKey) continue;
        await send(next);
        lastKey = key;
      }
    })().finally(() => { writing = null; });
    return writing;
  }

  function schedule() {
    if (frame || disposed) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      pending = collect();
      void pump().catch(onError);
    });
  }
  const resize = new ResizeObserver(schedule);
  resize.observe(document.documentElement);
  const mutations = new MutationObserver(schedule);
  mutations.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["style", "class"] });
  const capture = (event: PointerEvent) => {
    if (event.button === 0 && event.target instanceof Element && event.target.closest(".desktop-stack")) {
      // Retain the capture origin during a drag/resize, without ever expanding
      // to fullscreen. Pointer capture continues across the region's holes.
      captured = collect().rects;
    }
  };
  const release = () => { captured = []; schedule(); };
  document.addEventListener("pointerdown", capture, true);
  document.addEventListener("pointerup", release, true);
  document.addEventListener("pointercancel", release, true);
  document.addEventListener("scroll", schedule, true);
  window.addEventListener("resize", schedule);
  window.addEventListener("blur", release);
  schedule();

  return {
    async flush() {
      cancelAnimationFrame(frame); frame = 0;
      pending = collect();
      await pump();
    },
    dispose() {
      disposed = true; cancelAnimationFrame(frame); pending = null;
      resize.disconnect(); mutations.disconnect(); watched.clear();
      document.removeEventListener("pointerdown", capture, true);
      document.removeEventListener("pointerup", release, true);
      document.removeEventListener("pointercancel", release, true);
      document.removeEventListener("scroll", schedule, true);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("blur", release);
    },
  };
}
