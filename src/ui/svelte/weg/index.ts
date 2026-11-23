import { getRootContainer } from "libs/ui/react/utils/index.ts";
import { mount } from "svelte";
import App from "./App.svelte";
import { MacUiCommand, Widget } from "@macui/lib";
import { invoke } from "@tauri-apps/api/core";

import "./styles/variables.css";
import "@macui/lib/styles/reset.css";
import "./styles/global.css";
import { trackFrostedSurfaces } from "libs/ui/svelte/utils/frostedSurfaces";

await Widget.getCurrent().init();

mount(App, {
  target: getRootContainer(),
});
trackFrostedSurfaces(".taskbar.mac-dock-material");

Widget.self.window.onDragDropEvent(async (e) => {
  if (e.payload.type === "drop") {
    for (const path of e.payload.paths) {
      await invoke(MacUiCommand.WegPinItem, { path });
    }
  }
});
