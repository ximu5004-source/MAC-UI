import { mount } from "svelte";
import { invoke, SeelenCommand, Widget } from "@seelen-ui/lib";
import App from "./app.svelte";
import "@seelen-ui/lib/styles/reset.css";
import "libs/ui/styles/frosted-material.css";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

await Widget.self.init({ saveAndRestoreLastRect: false, normalizeDevicePixelRatio: true });
await Widget.self.window.setResizable(false);
await invoke(SeelenCommand.SetAsDesktop);
mount(App, { target: document.getElementById("root")! });
trackFrostedSurfaces();
