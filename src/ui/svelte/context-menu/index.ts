import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@macui/lib";

import "@macui/lib/styles/reset.css";
import "libs/ui/styles/frosted-material.css";
import "./glass-menu.css";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

const root = document.getElementById("root")!;

await Widget.self.init({
  autoSizeByContent: root,
});

mount(App, {
  target: root,
});
trackFrostedSurfaces(".context-menu", true);
