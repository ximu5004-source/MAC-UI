import { Widget } from "@macui/lib";
import type { PhysicalMonitor } from "@macui/lib/types";
import { globalState } from "./mod.svelte";
import { StartDisplayMode, StartView } from "../constants";
import { tick } from "svelte";
import { LatestPlacement } from "../launchpadOperations";
import { launchpadBounds } from "../launchpadGeometry";

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
  const rect = launchpadBounds(targetMonitor, globalState.displayMode === StartDisplayMode.Fullscreen);
  return rect ? placement.request(rect) : Promise.resolve();
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
  globalState.choosingFavorites = false;
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
