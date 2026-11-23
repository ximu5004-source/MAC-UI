import { getRootContainer } from "libs/ui/react/utils/index.ts";
import { mount } from "svelte";
import App from "./App.svelte";
import { Widget } from "@macui/lib";

import "./styles/variables.css";
import "@macui/lib/styles/reset.css";
import "./styles/global.css";

await Widget.getCurrent().init();

mount(App, {
  target: getRootContainer(),
});
