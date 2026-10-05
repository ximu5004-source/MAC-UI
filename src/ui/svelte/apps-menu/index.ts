import { getRootContainer } from "libs/ui/react/utils/index.ts";
import { mount } from "svelte";
import App from "./App.svelte";
import { Widget } from "@seelen-ui/lib";
import { onTriggered } from "./state/positioning.svelte.ts";
import { LatestToggle } from "./launchpadOperations";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/styles/frosted-material.css";
import "./launchpad.css";

const widget = Widget.getCurrent();

await widget.init({ hideOnFocusLoss: true });
await widget.window.setFocusable(true);

const visibility = new LatestToggle<{ x: number; y: number } | null | undefined>(
  () => widget.window.isVisible(),
  (visible, position, isCurrent) => visible ? onTriggered(position, isCurrent) : widget.hide(),
);

widget.onTrigger(async (args) => {
  await visibility.toggle(args.desiredPosition).catch(error => console.error("Launchpad toggle failed:", error));
});

mount(App, {
  target: getRootContainer(),
});
// Only the main surface samples pixels behind the native WebView. The folder
// dialog must instead blur the app content in this same WebView using CSS.
// Wallpaper alone cannot predict the pixels of an application behind this
// window. Keep Launchpad ink tied to the chosen light/dark appearance, instead
// of selecting black text from a bright wallpaper over a dark live backdrop.
// The toolbar's separately requested wallpaper-adaptive text is unchanged.
trackFrostedSurfaces(".launchpad.mac-frosted-surface");
