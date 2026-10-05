import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { toolbarForeground, type ToolbarForeground } from "./contrast";

/** One coalesced request; polling notices Windows slideshows without screen capture. */
export function watchWallpaperContrast() {
  const groups = ["left", "center", "right"] as const;
  const previous = new Map<string, ToolbarForeground>();
  let disposed = false, running = false, pending = false;
  let warned = false;
  async function refresh() {
    if (disposed || document.hidden) return;
    if (running) { pending = true; return; }
    running = true;
    try {
      const samples = groups.flatMap((id) => {
        const items = Array.from(document.querySelectorAll<HTMLElement>(`.ft-bar-${id} .ft-bar-item`))
          .map((item) => item.getBoundingClientRect()).filter((rect) => rect.width > 0 && rect.height > 0);
        if (!items.length) return [];
        return [{ id, rect: [Math.min(...items.map((r) => r.left)), Math.min(...items.map((r) => r.top)), Math.max(...items.map((r) => r.right)), Math.max(...items.map((r) => r.bottom))] }];
      });
      if (!samples.length) return;
      const luminances = await invoke(SeelenCommand.GetToolbarWallpaperLuminance, {
        viewport: [innerWidth, innerHeight], rects: samples.map((sample) => sample.rect as [number, number, number, number]),
      });
      if (disposed) return;
      samples.forEach(({ id }, index) => {
        const foreground = toolbarForeground(luminances[index]!, previous.get(id));
        previous.set(id, foreground);
        document.documentElement.style.setProperty(`--toolbar-${id}-fg`, foreground === "black" ? "#000" : "#fff");
        document.documentElement.style.setProperty(`--toolbar-${id}-shadow`, foreground === "black" ? "0 1px 2px rgb(255 255 255 / .2)" : "0 1px 3px rgb(0 0 0 / .65)");
      });
      warned = false;
    } catch (error) {
      // Keep the last good contrast instead of flashing the bar on a transient wallpaper read failure.
      if (!warned) console.warn("Could not refresh wallpaper contrast", error);
      warned = true;
    } finally {
      running = false;
      if (pending && !disposed) { pending = false; void refresh(); }
    }
  }
  const interval = setInterval(() => void refresh(), 2000);
  const observer = new MutationObserver(() => void refresh());
  const bar = document.querySelector(".ft-bar");
  if (bar) observer.observe(bar, { childList: true, subtree: true });
  const update = () => { void refresh(); };
  window.addEventListener("resize", update);
  document.addEventListener("visibilitychange", update);
  void refresh();
  return () => { disposed = true; clearInterval(interval); observer.disconnect(); window.removeEventListener("resize", update); document.removeEventListener("visibilitychange", update); };
}
