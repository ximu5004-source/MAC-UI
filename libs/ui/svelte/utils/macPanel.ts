import type { Widget } from "@seelen-ui/lib";
import { currentMonitor, monitorFromPoint } from "@tauri-apps/api/window";

/** Physical work area -> CSS size, independent of the auto-sized webview. */
export function macPanelBounds(width: number, height: number, scale = 1) {
  const dpi = Number.isFinite(scale) && scale > 0 ? scale : 1;
  return {
    width: Math.max(1, Math.floor((Number.isFinite(width) && width > 0 ? width : 420) / dpi - 20)),
    height: Math.max(1, Math.min(680, Math.floor((Number.isFinite(height) && height > 0 ? height : 720) / dpi - 32))),
  };
}

export async function configureMacPanel(widget: Widget) {
  async function resize(point?: { x: number; y: number } | null) {
    const monitor = await (point ? monitorFromPoint(point.x, point.y) : currentMonitor()).catch(() => null);
    const bounds = monitor
      ? macPanelBounds(monitor.workArea.size.width, monitor.workArea.size.height, monitor.scaleFactor)
      : macPanelBounds(screen.availWidth, screen.availHeight);
    document.documentElement.style.setProperty("--mac-panel-max-width", `${bounds.width}px`);
    document.documentElement.style.setProperty("--mac-panel-max-height", `${bounds.height}px`);
  }
  await resize();
  widget.onTrigger(({ desiredPosition }) => {
    void resize(desiredPosition);
  });
}
