import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { frostedRect, FrostedWriter, type FrostedSnapshot } from "./frostedGeometry";
import { frostedForeground, type FrostedInk } from "./frostedContrast";

/** Register actual painted surfaces, separately from desktop input regions. */
export function trackFrostedSurfaces(selector = ".mac-frosted-surface", adaptiveInk = false) {
  let disposed = false;
  let frame = 0;
  let motionUntil = 0;
  let reportedFailure = false;
  let paintedNodes: HTMLElement[] = [];
  let sampling = false, lastSample = 0, lastSampleKey = "";
  let sampleGeneration = 0;
  const watched = new Set<Element>();
  const reduced = matchMedia("(prefers-reduced-transparency: reduce), (forced-colors: active), (prefers-contrast: more)");
  const writer = new FrostedWriter(async (snapshot) => {
    await invoke(SeelenCommand.SetFrostedRegions, snapshot);
    document.documentElement.dataset.nativeFrosted = String(snapshot.rects.length > 0 && !disposed);
    reportedFailure = false;
  });

  function collect(): FrostedSnapshot {
    paintedNodes = [];
    const viewport: [number, number] = [innerWidth, innerHeight];
    const snapshot: FrostedSnapshot = { viewport, rects: [] };
    if (disposed || reduced.matches || document.documentElement.hasAttribute("data-widget-hidden")) return snapshot;
    for (const node of Array.from(document.querySelectorAll<HTMLElement>(selector))) {
      if (!watched.has(node)) { watched.add(node); resize.observe(node); }
      let visible = true;
      for (let parent: HTMLElement | null = node; parent; parent = parent.parentElement) {
        const css = getComputedStyle(parent);
        if (css.display === "none" || css.visibility === "hidden" || Number(css.opacity) === 0
          || parent.dataset.showing === "false") { visible = false; break; }
      }
      if (!visible) continue;
      const rect = frostedRect(node.getBoundingClientRect(), getComputedStyle(node).borderTopLeftRadius, viewport);
      if (rect) { snapshot.rects.push(rect); paintedNodes.push(node); }
    }
    for (const node of watched) {
      if (!node.isConnected) { resize.unobserve(node); watched.delete(node); }
    }
    snapshot.rects = snapshot.rects.slice(0, 512);
    return snapshot;
  }

  async function refreshInk(snapshot: FrostedSnapshot) {
    if (!adaptiveInk) return;
    if (!snapshot.rects.length) { lastSampleKey = ""; sampleGeneration++; return; }
    const key = JSON.stringify(snapshot);
    if (sampling || (key === lastSampleKey && performance.now() - lastSample < 2500)) return;
    sampling = true;
    const generation = ++sampleGeneration;
    const nodes = paintedNodes.slice(0, 3);
    try {
      const values = await invoke(SeelenCommand.GetFrostedWallpaperLuminance, {
        viewport: snapshot.viewport,
        rects: snapshot.rects.slice(0, 3).map(([l, t, r, b]) => [l, t, r, b] as [number, number, number, number]),
      });
      if (disposed || generation !== sampleGeneration) return;
      nodes.forEach((node, index) => {
        if (node.isConnected) node.dataset.frostedInk = frostedForeground(values[index]!, node.dataset.frostedInk as FrostedInk | undefined);
      });
      lastSampleKey = key;
    } catch {
      // Keep the last known foreground (or system theme) on wallpaper failure.
    } finally {
      lastSample = performance.now();
      sampling = false;
    }
  }

  function send(snapshot: FrostedSnapshot) {
    void writer.write(snapshot).catch((error) => {
      document.documentElement.dataset.nativeFrosted = "false";
      if (!reportedFailure) console.warn("Native frost unavailable; CSS preview cannot blur other windows:", error);
      reportedFailure = true;
    });
  }
  function schedule() {
    if (frame || disposed) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      const snapshot = collect();
      send(snapshot);
      void refreshInk(snapshot);
      if (performance.now() < motionUntil) schedule();
    });
  }
  // Follow only the short geometry transitions (Dock auto-hide), not an idle
  // animation loop. Descendant hover color transitions need no native rebuild.
  function followMotion(event: Event) {
    if (!(event.target instanceof Element)) return;
    const target = event.target;
    if (!target.matches(selector) && !target.querySelector(selector)) return;
    if (event instanceof TransitionEvent && !["transform", "translate", "width", "height", "opacity"].includes(event.propertyName)) return;
    motionUntil = performance.now() + 400;
    schedule();
  }
  const resize = new ResizeObserver(schedule);
  resize.observe(document.documentElement);
  const mutations = new MutationObserver(schedule);
  mutations.observe(document.documentElement, {
    childList: true, subtree: true, attributes: true,
    attributeFilter: ["style", "class", "data-widget-hidden", "data-showing"],
  });
  document.addEventListener("transitionrun", followMotion, true);
  document.addEventListener("transitionend", schedule, true);
  document.addEventListener("scroll", schedule, true);
  window.addEventListener("resize", schedule);
  reduced.addEventListener("change", schedule);
  const contrastPoll = adaptiveInk ? setInterval(schedule, 3000) : undefined;
  const recovery = getCurrentWebviewWindow().listen("mac-native-frost-recover", () => {
    document.documentElement.dataset.nativeFrosted = "false";
    writer.invalidate();
    schedule();
  }).catch((error) => {
    console.warn("Native frost recovery listener unavailable:", error);
    return () => {};
  });
  const dispose = () => {
    disposed = true;
    clearInterval(contrastPoll);
    cancelAnimationFrame(frame);
    resize.disconnect(); mutations.disconnect(); watched.clear();
    document.removeEventListener("transitionrun", followMotion, true);
    document.removeEventListener("transitionend", schedule, true);
    document.removeEventListener("scroll", schedule, true);
    window.removeEventListener("resize", schedule);
    window.removeEventListener("pagehide", dispose);
    reduced.removeEventListener("change", schedule);
    void recovery.then((unlisten) => unlisten());
    send({ viewport: [innerWidth, innerHeight], rects: [] });
  };
  window.addEventListener("pagehide", dispose);
  schedule();
  return { dispose };
}
