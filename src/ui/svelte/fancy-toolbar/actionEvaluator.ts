import type Sandbox from "@nyariv/sandboxjs";
import { Alignment, FancyToolbarSide, type WidgetId, type WidgetTriggerPayload } from "@macui/lib/types";
import { toPhysicalPixels } from "libs/ui/react/utils/index.ts";
import { invoke, MacUiCommand } from "@macui/lib";
import { settingsState } from "./state/settings.svelte.ts";
import { evalSanboxed } from "libs/ui/svelte/utils/sandbox.ts";

const ALLOWED_COMMANDS = [
  MacUiCommand.SwitchWorkspace,
  MacUiCommand.SetVolumeLevel,
  MacUiCommand.OpenFile,
  MacUiCommand.MediaPrev,
  MacUiCommand.MediaNext,
  MacUiCommand.MediaTogglePlayPause,
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
  open(path: string) {
    invoke(MacUiCommand.OpenFile, { path });
  },
  // trigger is added on other step but is present too
};

export function evalActionSanboxed(
  executor: ReturnType<Sandbox["compile"]> | null,
  scope: Record<string, any>,
) {
  evalSanboxed(executor, { ...scope, ...ActionsScope });
}

export function triggerWidget(
  widgetId: WidgetId,
  itemId: string,
  customArgs?: WidgetTriggerPayload["customArgs"],
): void {
  if (typeof widgetId !== "string") {
    return;
  }

  const { left: windowX, top: windowY } = settingsState.widgetRect;

  const element = document.getElementById(itemId);
  if (!element) {
    console.error(`Element with id ${itemId} not found`);
    return;
  }

  const domRect = element.getBoundingClientRect();
  const x = windowX + toPhysicalPixels(domRect.left + domRect.width / 2);

  const rootRect = document.getElementById("root")!.getBoundingClientRect();
  const isTopPosition = settingsState.position === FancyToolbarSide.Top;

  const y = isTopPosition ? windowY + toPhysicalPixels(rootRect.bottom) : windowY + toPhysicalPixels(rootRect.top);

  invoke(MacUiCommand.TriggerWidget, {
    payload: {
      id: widgetId,
      customArgs: {
        ...customArgs,
        // Popup preset alone consumes this key. Hover previews and other
        // programmatic trigger sources retain their show/update semantics.
        __macuiPopupToggleSource: `toolbar:${itemId}:${windowX}:${windowY}`,
      },
      desiredPosition: { x, y },
      alignX: Alignment.Center,
      alignY: isTopPosition ? Alignment.Start : Alignment.End,
    },
  });
}
