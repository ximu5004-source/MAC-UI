import { mount } from "svelte";
import App from "./app.svelte";
import { Settings, Widget } from "@seelen-ui/lib";
import { locale } from "libs/ui/svelte/utils/i18n";
import { currentMonitor, monitorFromPoint } from "@tauri-apps/api/window";
import { trayPanelSize } from "./trayPresentation";
import { state } from "./state.svelte";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/svelte/styles/mac-panels.css";
import { configureMacPanel } from "libs/ui/svelte/utils/macPanel";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

const root = document.getElementById("root")!;

const widget = Widget.getCurrent();
await locale.set((await Settings.getAsync()).language);
Settings.onChange((settings) => locale.set(settings.language));
await widget.init({
  autoSizeByContent: root,
});
await configureMacPanel(widget);

async function updatePanelSize(point?: { x: number; y: number } | null) {
  const monitor = await (point ? monitorFromPoint(point.x, point.y) : currentMonitor()).catch(() => null);
  const size = monitor
    ? trayPanelSize(monitor.workArea.size.width, monitor.workArea.size.height, monitor.scaleFactor)
    : trayPanelSize(window.screen.availWidth, window.screen.availHeight);
  root.style.setProperty("--tray-width", `${size.width}px`);
  root.style.setProperty("--tray-max-height", `${size.maxHeight}px`);
}

await updatePanelSize();
widget.onTrigger(({ desiredPosition }) => {
  void updatePanelSize(desiredPosition);
  void state.refresh();
});

mount(App, {
  target: root,
});
trackFrostedSurfaces(".mac-frosted-surface", true);
