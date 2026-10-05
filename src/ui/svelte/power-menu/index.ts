import { getRootContainer } from "libs/ui/react/utils/index.ts";
import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@seelen-ui/lib";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/styles/frosted-material.css";
import "./power-menu.css";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

const widget = Widget.getCurrent();
widget.onTrigger(async () => {
  await widget.show();
  await widget.focus();
  window.dispatchEvent(new Event("mac-ui::power-opened"));
});

await widget.init({ normalizeDevicePixelRatio: true, hideOnFocusLoss: true });
await widget.window.setResizable(false);

mount(App, {
  target: getRootContainer(),
});
// The wallpaper alone cannot predict the applications below a session dialog.
// Keep foreground in the chosen/system appearance, and frost only the sheet.
trackFrostedSurfaces(".power-sheet.mac-frosted-surface");
