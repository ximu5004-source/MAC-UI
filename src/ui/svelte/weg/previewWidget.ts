import { invoke, MacUiCommand } from "@macui/lib";
import { Alignment, MacUiWegSide, type UserAppWindow, type WidgetId } from "@macui/lib/types";
import { settingsState } from "./state/settings.svelte.ts";
import { captureDockTarget } from "./dockTarget";

export async function triggerPreviewWidget(itemEl: HTMLElement, windows: UserAppWindow[], hovered?: () => boolean) {
  const dockSide = settingsState.position;

  const elRect = itemEl.getBoundingClientRect();
  const { hitboxRect: viewRect, webviewRect } = settingsState.widgetRect;

  const toPhysical = (n: number) => Math.round(n * globalThis.devicePixelRatio);

  const anchor = {
    left: webviewRect.left + toPhysical(elRect.left),
    right: webviewRect.left + toPhysical(elRect.right),
    top: webviewRect.top + toPhysical(elRect.top),
    bottom: webviewRect.top + toPhysical(elRect.bottom),
  };
  let x: number;
  let y: number;
  let alignX: Alignment;
  let alignY: Alignment;

  switch (dockSide) {
    case MacUiWegSide.Bottom:
      x = Math.round((anchor.left + anchor.right) / 2);
      y = viewRect.top;
      alignX = Alignment.Center;
      alignY = Alignment.End;
      break;
    case MacUiWegSide.Top:
      x = Math.round((anchor.left + anchor.right) / 2);
      y = viewRect.bottom;
      alignX = Alignment.Center;
      alignY = Alignment.Start;
      break;
    case MacUiWegSide.Left:
      x = viewRect.right;
      y = Math.round((anchor.top + anchor.bottom) / 2);
      alignX = Alignment.Start;
      alignY = Alignment.Center;
      break;
    case MacUiWegSide.Right:
      x = viewRect.left;
      y = Math.round((anchor.top + anchor.bottom) / 2);
      alignX = Alignment.End;
      alignY = Alignment.Center;
      break;
  }

  const transitionTarget = await captureDockTarget(itemEl);
  if (hovered && !hovered()) return;
  await invoke(MacUiCommand.TriggerWidget, {
    payload: {
      id: "@macui/weg-preview" as WidgetId,
      desiredPosition: { x, y },
      alignX,
      alignY,
      customArgs: {
        hwnds: windows.map((w) => w.hwnd),
        position: dockSide,
        hoverAnchor: hovered ? anchor : null,
        minimizeTarget: transitionTarget,
      },
    },
  });
}
