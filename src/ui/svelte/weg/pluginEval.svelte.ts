import type Sandbox from "@nyariv/sandboxjs";
import { Alignment, MacUiWegSide, type WidgetId } from "@macui/lib/types";
import { invoke, MacUiCommand } from "@macui/lib";
import { settingsState } from "./state/settings.svelte.ts";
import { evalSanboxed } from "libs/ui/svelte/utils/sandbox.ts";

const ALLOWED_COMMANDS: MacUiCommand[] = [
  MacUiCommand.OpenFile,
  MacUiCommand.ShowDesktop,
  MacUiCommand.ShowStartMenu,
];

const ActionsScope = {
  MacUiCommand,
  invoke(command: MacUiCommand, args?: any) {
    if (!ALLOWED_COMMANDS.includes(command)) {
      console.warn(`Trying to execute command that is not allowed: "${command}"`);
      return;
    }
    invoke(command, args);
  },
};

export function evalActionSanboxed(
  executor: ReturnType<Sandbox["compile"]> | null,
  scope: Record<string, any>,
): void {
  evalSanboxed(executor, { ...scope, ...ActionsScope });
}

export function triggerWidget(widgetId: WidgetId, itemId: string): void {
  if (typeof widgetId !== "string") {
    return;
  }

  const element = document.getElementById(itemId);
  if (!element) {
    console.error(`Element with id ${itemId} not found`);
    return;
  }

  const dockSide = settingsState.position;
  const elRect = element.getBoundingClientRect();
  const viewRect = settingsState.widgetRect.hitboxRect;

  const toPhysical = (n: number) => Math.round(n * globalThis.devicePixelRatio);

  let x: number;
  let y: number;
  let alignX: Alignment;
  let alignY: Alignment;

  switch (dockSide) {
    case MacUiWegSide.Bottom:
      x = viewRect.left + toPhysical(elRect.left + elRect.width / 2);
      y = viewRect.top;
      alignX = Alignment.Center;
      alignY = Alignment.End;
      break;
    case MacUiWegSide.Top:
      x = viewRect.left + toPhysical(elRect.left + elRect.width / 2);
      y = viewRect.bottom;
      alignX = Alignment.Center;
      alignY = Alignment.Start;
      break;
    case MacUiWegSide.Left:
      x = viewRect.right;
      y = viewRect.top + toPhysical(elRect.top + elRect.height / 2);
      alignX = Alignment.Start;
      alignY = Alignment.Center;
      break;
    case MacUiWegSide.Right:
      x = viewRect.left;
      y = viewRect.top + toPhysical(elRect.top + elRect.height / 2);
      alignX = Alignment.End;
      alignY = Alignment.Center;
      break;
  }

  invoke(MacUiCommand.TriggerWidget, {
    payload: {
      id: widgetId,
      desiredPosition: { x, y },
      alignX,
      alignY,
    },
  });
}
