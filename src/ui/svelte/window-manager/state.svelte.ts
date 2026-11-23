import { invoke, RuntimeStyleSheet, MacUiCommand, MacUiEvent, Settings, subscribe, Widget } from "@macui/lib";
import { declareDocumentAsLayeredHitbox } from "libs/ui/react/utils/layered";
import type { FocusedApp, TwmReservation, TwmRuntimeTree, WindowManagerSettings } from "@macui/lib/types";
import { FancyToolbarSide, HideMode } from "@macui/lib/types";
import { MacUiWegSide } from "node_modules/@macui/lib/esm/gen/types/MacUiWegSide";

import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";
import { isTouchPrimary } from "libs/ui/svelte/utils/signals.svelte.ts";

let layouts = lazyRune(() => invoke(MacUiCommand.WmGetRenderTree));
subscribe(MacUiEvent.WMTreeChanged, layouts.setByPayload);

let workspaces = lazyRune(() => invoke(MacUiCommand.StateGetVirtualDesktops));
subscribe(MacUiEvent.VirtualDesktopsChanged, workspaces.setByPayload);

let interactables = lazyRune(() => invoke(MacUiCommand.GetUserAppWindows));
subscribe(MacUiEvent.UserAppWindowsChanged, interactables.setByPayload);

let monitors = lazyRune(() => invoke(MacUiCommand.SystemGetMonitors));
subscribe(MacUiEvent.SystemMonitorsChanged, monitors.setByPayload);

let reservation = $state<TwmReservation | null>(null);
subscribe(MacUiEvent.WMSetReservation, (e) => {
  reservation = e.payload;
});

let forceRepositioning = $state(0);
subscribe(MacUiEvent.WMForceRetiling, () => {
  forceRepositioning++;
});

const [focusedAppInit, settingsInit] = await Promise.all([
  invoke(MacUiCommand.GetFocusedApp),
  Settings.getAsync(),
  layouts.init(),
  workspaces.init(),
  interactables.init(),
  monitors.init(),
]);

let focusedApp = $state<FocusedApp>(focusedAppInit);
subscribe(MacUiEvent.GlobalFocusChanged, (e) => {
  focusedApp = e.payload;
});

let fullSettings = $state(settingsInit);
let settings = $state<WindowManagerSettings>(settingsInit.byWidget["@macui/window-manager"]);
Settings.onChange((s) => {
  fullSettings = s;
  settings = s.byWidget["@macui/window-manager"];
});

// =================================================
//                  CSS variables
// =================================================

$effect.root(() => {
  $effect(() => {
    const sheet = new RuntimeStyleSheet("@config/window-manager");
    sheet.addVariable("--config-padding", `${settings.workspacePadding}px`);
    sheet.addVariable("--config-containers-gap", `${settings.workspaceGap}px`);
    sheet.addVariable("--config-margin-top", `${settings.workspaceMargin.top}px`);
    sheet.addVariable("--config-margin-left", `${settings.workspaceMargin.left}px`);
    sheet.addVariable("--config-margin-right", `${settings.workspaceMargin.right}px`);
    sheet.addVariable("--config-margin-bottom", `${settings.workspaceMargin.bottom}px`);
    sheet.addVariable("--config-border-offset", `${settings.border.offset}px`);
    sheet.addVariable("--config-border-width", `${settings.border.width}px`);
    sheet.applyToDocument();
  });
});

// =================================================
//                   Positioning
// =================================================

const monitorId = Widget.getCurrent().decoded.monitorId!;

const widgetRect = $derived.by(() => {
  const monitor = monitors.value.find((m) => m.id === monitorId);
  if (!monitor) {
    throw new Error("Current monitor not found");
  }

  const rect = { ...monitor.rect };
  const tbConfig = fullSettings.byWidget["@macui/fancy-toolbar"];
  const tbMonitorConfig = (fullSettings.monitorsV3[monitor.id] as any)?.byWidget?.[
    "@macui/fancy-toolbar"
  ] ?? {
    enabled: true,
  };

  if (
    tbConfig.enabled &&
    tbMonitorConfig.enabled &&
    (tbConfig.hideMode === HideMode.Never || isTouchPrimary.value)
  ) {
    const tbSize = Math.round(
      (tbConfig.itemSize + tbConfig.padding * 2 + tbConfig.margin * 2) * monitor.scaleFactor,
    );
    switch (tbConfig.position) {
      case FancyToolbarSide.Top:
        rect.top += tbSize;
        break;
      case FancyToolbarSide.Bottom:
        rect.bottom -= tbSize;
        break;
    }
  }

  const wegConfig = fullSettings.byWidget["@macui/weg"];
  const wegMonitorConfig = (fullSettings.monitorsV3[monitor.id] as any)?.byWidget?.[
    "@macui/weg"
  ] ?? {
    enabled: true,
  };

  if (
    wegConfig.enabled &&
    wegMonitorConfig.enabled &&
    (wegConfig.hideMode === HideMode.Never || isTouchPrimary.value)
  ) {
    const wegSize = Math.round(
      (wegConfig.size + wegConfig.padding * 2 + wegConfig.margin * 2) * monitor.scaleFactor,
    );
    switch (wegConfig.position) {
      case MacUiWegSide.Top:
        rect.top += wegSize;
        break;
      case MacUiWegSide.Bottom:
        rect.bottom -= wegSize;
        break;
      case MacUiWegSide.Left:
        rect.left += wegSize;
        break;
      case MacUiWegSide.Right:
        rect.right -= wegSize;
        break;
    }
  }

  return rect;
});

$effect.root(() => {
  Widget.self.attachPosition();
  $effect(() => {
    Widget.self.setPosition(widgetRect);
  });
});

await declareDocumentAsLayeredHitbox({
  getPhysicalRect: () => {
    const r = widgetRect;
    return { x: r.left, y: r.top, width: r.right - r.left, height: r.bottom - r.top };
  },
  shouldAllowMouseEvent: (e) => e.getAttribute("data-allow-mouse-events") === "true",
});

// =================================================
//               Exported State Getters
// =================================================

export type State = _State;
class _State {
  readonly paused = $derived.by(
    () => layouts.value.paused || !!layouts.value.pausedByMonitor[monitorId],
  );

  getLayout(monitorId: string): TwmRuntimeTree | null {
    const activeWsId = workspaces.value?.monitors?.[monitorId]?.active_workspace;
    if (!activeWsId) return null;
    return layouts.value?.workspaces?.[activeWsId] ?? null;
  }
  get forceRepositioning() {
    return forceRepositioning;
  }
  get interactables() {
    return interactables.value;
  }
  get focusedApp() {
    return focusedApp;
  }
  get settings() {
    return settings;
  }
  get reservation() {
    return reservation;
  }
  get widgetRect() {
    return widgetRect;
  }
}

export const state = new _State();
