import { getRootContainer } from "libs/ui/react/utils/index.ts";
import { createRoot } from "react-dom/client";
import { I18nextProvider } from "react-i18next";
import { HashRouter } from "react-router";
import { Widget } from "@seelen-ui/lib";
import { LogicalSize } from "@seelen-ui/lib/tauri";

import { App } from "./app.tsx";

import i18n from "./i18n/index.ts";

import "./styles/variables.css";
import "@seelen-ui/lib/styles/reset.css";
import "./styles/global.css";
import "./styles/RichText.css";

const { window } = Widget.self;

await Promise.all([
  window.setDecorations(false),
  window.setSizeConstraints({ minWidth: 720, minHeight: 520 }),
  window.setSize(new LogicalSize(1000, 740)),
  window.setTitle("MAC UI"),
]);
await window.center();

await Widget.self.init({ useThemes: false });
await Widget.self.show();

Widget.self.onTrigger((payload) => {
  window.unminimize();
  window.setFocus();
  const route = payload.customArgs?.["route"] as string | undefined;
  if (route) {
    globalThis.location.hash = route;
  }
});

const container = getRootContainer();
createRoot(container).render(
  <I18nextProvider i18n={i18n}>
    <HashRouter>
      <App />
    </HashRouter>
  </I18nextProvider>,
);
