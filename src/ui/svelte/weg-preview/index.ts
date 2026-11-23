import { mount, tick } from "svelte";
import { preparePreview } from "./state.svelte";
import App from "./App.svelte";
import { Widget } from "@macui/lib";

import "@macui/lib/styles/reset.css";

const root = document.getElementById("root")!;

await Widget.self.init({
  autoSizeByContent: root,
  hideOnFocusLoss: true,
  focusPopupOnShow: ({ customArgs }) => !customArgs?.hoverAnchor,
  async beforePopupShow(payload) {
    preparePreview(payload);
    await tick();
  },
});

mount(App, {
  target: root,
});
