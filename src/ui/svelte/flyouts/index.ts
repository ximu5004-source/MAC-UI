import { mount } from "svelte";
import App from "./app.svelte";
import { Widget } from "@seelen-ui/lib";

import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/styles/frosted-material.css";
import "./frosted-flyouts.css";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

const root = document.getElementById("root")!;

await Widget.self.init({
  autoSizeByContent: root,
});

mount(App, {
  target: root,
});
trackFrostedSurfaces('.flyout[data-showing="true"]', true);
