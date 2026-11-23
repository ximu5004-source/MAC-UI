import { invoke, MacUiCommand, MacUiEvent, subscribe, Widget } from "@macui/lib";
import { ZOrder } from "@macui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import { ConfigState } from "./config.svelte";
import { PhysicalPosition } from "@tauri-apps/api/window";

let showing = $state(false);
let notificationShowing = $state(false);

const monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

const widgetSize = lazyRune(() => Widget.self.window.outerSize());
await Widget.self.window.onResized(widgetSize.setByPayload);

await Promise.all([monitors.init(), widgetSize.init()]);

const primaryMonitor = $derived.by(() => {
  return monitors.value.find((m) => m.isPrimary) || monitors.value[0];
});

$effect.root(() => {
  $effect(() => {
    const monitor = primaryMonitor;
    if (!monitor) {
      return;
    }

    const placement = ConfigState.config.placement;
    const padding = ConfigState.config.margin * window.devicePixelRatio;

    const monitorWidth = monitor.rect.right - monitor.rect.left;
    const monitorHeight = monitor.rect.bottom - monitor.rect.top;

    const monitorCenterX = monitor.rect.left + monitorWidth / 2;
    const monitorCenterY = monitor.rect.top + monitorHeight / 2;

    const { width, height } = widgetSize.value;

    let x: number, y: number;
    if (placement === "left") {
      x = monitor.rect.left + padding;
      y = Math.round(monitorCenterY - height / 2);
    } else if (placement === "top") {
      x = Math.round(monitorCenterX - width / 2);
      y = monitor.rect.top + padding;
    } else if (placement === "right") {
      x = monitor.rect.right - width - padding;
      y = Math.round(monitorCenterY - height / 2);
    } else if (placement === "top-left") {
      x = monitor.rect.left + padding;
      y = monitor.rect.top + padding;
    } else if (placement === "top-right") {
      x = monitor.rect.right - width - padding;
      y = monitor.rect.top + padding;
    } else if (placement === "bottom-left") {
      x = monitor.rect.left + padding;
      y = monitor.rect.bottom - height - padding;
    } else if (placement === "bottom-right") {
      x = monitor.rect.right - width - padding;
      y = monitor.rect.bottom - height - padding;
    } else {
      x = Math.round(monitorCenterX - width / 2);
      y = monitor.rect.bottom - height - padding;
    }

    Widget.self.window.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)));
  });

  $effect(() => {
    Widget.self.window.setIgnoreCursorEvents(!showing);
    if (showing) {
      invoke(MacUiCommand.SetSelfZOrder, { zOrder: notificationShowing ? ZOrder.TopMost : ZOrder.Top });
    }
  });
});

export const RendererState = {
  set notificationShowing(value: boolean) {
    notificationShowing = value;
  },
  get all() {
    return monitors.value;
  },
  get primary() {
    return primaryMonitor;
  },
  get showing() {
    return showing;
  },
  set showing(value: boolean) {
    showing = value;
  },
};
