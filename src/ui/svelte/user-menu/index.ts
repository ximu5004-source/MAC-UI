import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@seelen-ui/lib";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/svelte/styles/mac-panels.css";
import "./user-menu.css";
import { configureMacPanel } from "libs/ui/svelte/utils/macPanel";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

const root = document.getElementById("root")!;

const widget = Widget.getCurrent();
await widget.init({
  autoSizeByContent: root,
});
await configureMacPanel(widget);

mount(App, {
  target: root,
});
// Account popovers share the native frost, but keep the system appearance ink.
// Wallpaper-only luminance cannot predict other windows behind this menu.
trackFrostedSurfaces(".user-popup.mac-frosted-surface");
