import { mount } from "svelte";
import App from "./App.svelte";
import { Widget } from "@macui/lib";

import "@macui/lib/styles/reset.css";

const root = document.getElementById("root")!;

const widget = Widget.getCurrent();
await widget.init({ normalizeDevicePixelRatio: true });
await widget.window.setFocusable(true);

mount(App, {
  target: root,
});
