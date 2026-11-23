import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@macui/lib";

import "@macui/lib/styles/reset.css";

const root = document.getElementById("root")!;

await Widget.self.init({
  autoSizeByContent: root,
});

await Widget.self.window.setIgnoreCursorEvents(true);

mount(App, {
  target: root,
});
