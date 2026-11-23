import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@macui/lib";
import { LogicalSize } from "@macui/lib/tauri";

import "@macui/lib/styles/reset.css";

await Widget.self.init();
await Promise.all([
  Widget.self.window.setSizeConstraints({ minWidth: 600, minHeight: 400 }),
  Widget.self.window.setSize(new LogicalSize(800, 500)),
  Widget.self.window.center(),
]);

mount(App, {
  target: document.getElementById("root")!,
});
