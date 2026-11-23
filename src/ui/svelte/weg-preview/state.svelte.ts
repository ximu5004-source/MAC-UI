import { invoke, MacUiCommand, MacUiEvent, subscribe, Widget } from "@macui/lib";
import { MacUiWegSide, type WidgetTriggerPayload, type UserAppWindow, type Rect } from "@macui/lib/types";
import { lazyRune } from "libs/ui/svelte/utils";
import { readHoverAnchor } from "./hoverBounds.ts";
import { readMinimizeTarget } from "../weg/minimizeTarget";

const interactables = lazyRune<UserAppWindow[]>(() => invoke(MacUiCommand.GetUserAppWindows));
subscribe(MacUiEvent.UserAppWindowsChanged, interactables.setByPayload);

const previews = lazyRune<Record<number, { data: string }>>(() => invoke(MacUiCommand.GetUserAppWindowsPreviews));
subscribe(MacUiEvent.UserAppWindowsPreviewsChanged, previews.setByPayload);

await Promise.all([interactables.init(), previews.init()]);

let hwnds = $state<number[]>([]);
let position = $state<MacUiWegSide>(MacUiWegSide.Bottom);
let transitionTarget = $state<Rect | null>(null);

let hoverAnchor = $state<Rect | null>(null);

export function preparePreview({ customArgs }: WidgetTriggerPayload) {
  hoverAnchor = readHoverAnchor(customArgs?.hoverAnchor);
  hwnds = (customArgs?.hwnds as number[]) ?? [];
  position = (customArgs?.position as MacUiWegSide) ?? MacUiWegSide.Bottom;
  transitionTarget = readMinimizeTarget(customArgs?.minimizeTarget);
}

const currentInteractables = $derived(interactables.value?.filter((w) => hwnds.includes(w.hwnd)) ?? []);

class PreviewState {
  get hoverAnchor() { return hoverAnchor; }
  get transitionTarget() { return transitionTarget; }
  get currentInteractables() {
    return currentInteractables;
  }

  get previews() {
    return previews;
  }

  get position() {
    return position;
  }
}

export const previewState = new PreviewState();
