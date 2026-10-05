import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@seelen-ui/lib";
import "moment/min/locales";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/svelte/styles/mac-panels.css";
import "./calendar.css";
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
// Wallpaper luminance cannot predict a dark application behind this popup.
// Keep the foreground aligned with the selected system appearance instead.
trackFrostedSurfaces(".calendar-popup.mac-frosted-surface");
