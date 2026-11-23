import { invoke, MacUiCommand } from "@macui/lib";
import { window as TauriWindow } from "@macui/lib/tauri";
import type { Rect } from "@macui/lib/types";
import { isDockTranslation, restingDockIconTarget } from "./animationTargetGeometry";

/** One measured batch per Dock, rather than a native position request per icon. */
class DockAnimationTargets {
  private dock: HTMLElement | null = null;
  private entries = new Map<HTMLElement, readonly number[]>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private busy = false;
  private rerun = false;
  private generation = 0;
  private lastPayload = "";
  private lastDiagnostic = "";

  register(element: HTMLElement, windows: readonly number[]) {
    this.entries.set(element, [...windows]);
    this.schedule();
    return () => {
      this.entries.delete(element);
      this.schedule();
    };
  }

  schedule = () => {
    // A continuous stream of layout/style changes must not postpone the first
    // registration forever. Further changes join the existing fixed batch.
    if (this.timer !== undefined) return;
    this.timer = setTimeout(() => {
      this.timer = undefined;
      void this.flush();
    }, 32);
  };

  attach = (dock: HTMLElement) => {
    this.dock = dock;
    this.generation++;
    this.lastPayload = "";
    const root = dock.closest<HTMLElement>("#root") ?? dock;
    const resize = new ResizeObserver(this.schedule);
    resize.observe(dock);
    const mutation = new MutationObserver(this.schedule);
    mutation.observe(root, { subtree: true, childList: true, attributes: true, attributeFilter: ["class", "style"] });
    const media = globalThis.matchMedia("(prefers-reduced-motion: reduce)");
    media.addEventListener("change", this.schedule);
    root.addEventListener("scroll", this.schedule, true);
    root.addEventListener("transitionend", this.schedule, true);
    root.addEventListener("animationend", this.schedule, true);
    globalThis.addEventListener("resize", this.schedule);
    const nativeWindow = TauriWindow.getCurrentWindow();
    const listeners = [
      nativeWindow.onMoved(this.schedule),
      nativeWindow.onResized(this.schedule),
      nativeWindow.onScaleChanged(this.schedule),
    ];
    this.schedule();
    return () => {
      if (this.dock === dock) {
        this.dock = null;
        this.generation++;
      }
      resize.disconnect();
      mutation.disconnect();
      media.removeEventListener("change", this.schedule);
      root.removeEventListener("scroll", this.schedule, true);
      root.removeEventListener("transitionend", this.schedule, true);
      root.removeEventListener("animationend", this.schedule, true);
      globalThis.removeEventListener("resize", this.schedule);
      for (const listener of listeners) void listener.then((unlisten) => unlisten()).catch(console.error);
      this.schedule();
    };
  };

  private async flush() {
    if (this.busy) { this.rerun = true; return; }
    this.busy = true;
    const generation = this.generation;
    try {
      const targets: Record<number, Rect> = {};
      let reason = "dock-disconnected";
      let entries = 0;
      const windows = new Set<number>();
      let originKey = "";
      const dock = this.dock;
      if (dock?.isConnected) {
        for (const [item, hwnds] of this.entries) {
          if (!item.isConnected || !dock.contains(item) || item.closest("[inert]")) continue;
          entries++;
          for (const hwnd of hwnds) if (Number.isSafeInteger(hwnd) && hwnd !== 0) windows.add(hwnd);
        }
        reason = "dock-connected";
      }
      // Geometry is also needed when the explicit Genie preference replaces
      // Windows animation. A system reduced-motion signal must not erase it.
      // The native backend owns the user's selected animation policy.
      if (dock?.isConnected) {
        const root = dock.closest<HTMLElement>("#root") ?? dock;
        const transform = new DOMMatrixReadOnly(getComputedStyle(root).transform);
        // Only the structural hide translation is inverted. Other transforms
        // cannot be interpreted as the native resting icon position safely.
        reason = "unsupported-root-transform";
        if (isDockTranslation(transform)) {
          const origin = await this.readOrigin();
          if (generation !== this.generation) { this.rerun = true; return; }
          originKey = `${origin.x},${origin.y}`;
          for (const [item, windows] of this.entries) {
            if (!item.isConnected || !dock.contains(item) || item.closest("[inert]")) continue;
            const icon = item.querySelector(".weg-item-icon") ?? item;
            const rect = restingDockIconTarget(icon.getBoundingClientRect(), origin,
              globalThis.devicePixelRatio, { x: transform.e, y: transform.f });
            if (!rect) continue;
            for (const hwnd of windows) {
              if (Number.isSafeInteger(hwnd) && hwnd !== 0) targets[hwnd] = rect;
            }
          }
          reason = Object.keys(targets).length ? "measured" : windows.size ? "unmeasurable-icons" : "no-windows";
        }
      }
      const diagnostic = JSON.stringify({ entries, windows: windows.size, measured: Object.keys(targets).length, reason });
      if (diagnostic !== this.lastDiagnostic) {
        console.debug("Dock animation target state", diagnostic);
        this.lastDiagnostic = diagnostic;
      }
      const payload = JSON.stringify([originKey, targets]);
      if (payload !== this.lastPayload && generation === this.generation) {
        await invoke(MacUiCommand.WegSetWindowAnimationTargets, { targets });
        this.lastPayload = payload;
      }
    } catch (error) {
      console.warn("Dock animation positions unavailable", error);
    } finally {
      this.busy = false;
      if (this.rerun) { this.rerun = false; this.schedule(); }
    }
  }

  private async readOrigin() {
    let timeout: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        TauriWindow.getCurrentWindow().innerPosition(),
        new Promise<never>((_, reject) => {
          timeout = setTimeout(() => reject(new Error("Dock geometry timeout")), 100);
        }),
      ]);
    } finally { clearTimeout(timeout); }
  }
}

const animationTargets = new DockAnimationTargets();
export const attachDockAnimationTargets = animationTargets.attach;
export function registerWindowAnimationTarget(element: HTMLElement, windows: readonly number[]) {
  return animationTargets.register(element, windows);
}
