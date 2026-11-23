import { mount } from "svelte";
import { Widget } from "@macui/lib";
import { getRootContainer } from "libs/ui/react/utils";

import App from "./App.svelte";

import "@macui/lib/styles/reset.css";
import "./styles/global.css";

const widget = Widget.getCurrent();
await widget.init();

const container = getRootContainer();
mount(App, {
  target: container,
});
