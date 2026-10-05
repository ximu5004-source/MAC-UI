import { Widget } from "@seelen-ui/lib";
import type { PhysicalMonitor } from "@seelen-ui/lib/types";
import { globalState } from "./mod.svelte";
import { StartDisplayMode, StartView } from "../constants";
import { tick } from "svelte";
import { LatestPlacement } from "../launchpadOperations";

let desiredPosition = $state<{ x: number; y: number } | null>(null);

// Monitor under the cursor position that triggered the menu, falling back to primary
let monitorToShow = $derived.by(() => {
  const pos = desiredPosition;
  let targetMonitor: PhysicalMonitor | undefined;

  if (pos) {
    targetMonitor = globalState.monitors.find(
      (m) =>
        m.rect.left <= pos.x &&
        pos.x < m.rect.right &&
        m.rect.top <= pos.y &&
        pos.y < m.rect.bottom,
    );
  }

  // Fallback to primary monitor if not found or not specified
  if (!targetMonitor) {
    targetMonitor = globalState.monitors.find((m) => m.isPrimary) || globalState.monitors[0];
  }

  return targetMonitor;
});

const placement = new LatestPlacement<{ left: number; top: number; right: number; bottom: number }>(
  rect => Widget.getCurrent().setPosition(rect),
);

function placeCenteredToMonitor(targetMonitor: PhysicalMonitor): Promise<void> {
  const monitorWidth = targetMonitor.rect.right - targetMonitor.rect.left;
  const monitorHeight = targetMonitor.rect.bottom - targetMonitor.rect.top;
  if (monitorWidth <= 0 || monitorHeight <= 0) return Promise.resolve();

  // globalState.displayMode === StartDisplayMode.Fullscreen
  let x = targetMonitor.rect.left;
  let y = targetMonitor.rect.top;
  let width = monitorWidth;
  let height = monitorHeight;

  if (globalState.displayMode === StartDisplayMode.Normal) {
    // Wide, compact application surface. Keep the existing fullscreen preference
    // and monitor/DPI selection, while leaving visible desktop around windowed UI.
    width = Math.round(Math.min(monitorWidth * 0.9, 1100 * targetMonitor.scaleFactor));
    height = Math.round(Math.min(monitorHeight * 0.88, 850 * targetMonitor.scaleFactor));

    const monitorCenterX = targetMonitor.rect.left + monitorWidth / 2;
    const monitorCenterY = targetMonitor.rect.top + monitorHeight / 2;

    x = Math.round(monitorCenterX - width / 2);
    y = Math.round(monitorCenterY - height / 2);
  }

  return placement.request({
    left: x,
    top: y,
    right: x + width,
    bottom: y + height,
  });
}

$effect.root(() => {
  $effect(() => {
    globalState.displayMode;
    if (monitorToShow) {
      void placeCenteredToMonitor(monitorToShow).catch(error => console.error("Launchpad placement failed:", error));
    }
  });
});

export async function onTriggered(cursorPosition?: { x: number; y: number } | null, isCurrent = () => true) {
  if (!isCurrent()) return;
  desiredPosition = cursorPosition ?? null;

  globalState.view = StartView.Favorites;
  globalState.page = 0;
  globalState.openFolderId = null;
  globalState.version++; // trigger reactive updates
  // The live native backdrop follows the actual desktop; no static wallpaper
  // copy is needed for this surface.
  // Let the geometry effect enqueue the latest cursor/monitor/display-mode target.
  // Always re-place on open as native geometry can have changed while the view slept.
  await tick();
  if (monitorToShow) await placeCenteredToMonitor(monitorToShow);
  await placement.settled();
  if (!isCurrent()) return;
  await Widget.self.show();
  if (!isCurrent()) return;
  await Widget.self.focus();
}
