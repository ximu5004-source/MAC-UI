import { ConfigProvider } from "antd";
import { useEffect } from "react";
import { Widget } from "@macui/lib";

import { SettingsDialogHost } from "./components/SettingsDialogs";
import { Routing } from "./router.tsx";
import { ThumbnailGeneratorModal } from "./components/ThumbnailGeneratorModal/index.tsx";
import { WelcomeModal } from "./components/WelcomeModal/infra.tsx";
import { colorScheme } from "./state/colorScheme";

export function App() {
  const enhanced = colorScheme.value === "dark";

  useEffect(() => {
    setTimeout(() => {
      let splashscreen = document.getElementById("splashscreen");
      splashscreen?.classList.add("vanish");
      setTimeout(() => splashscreen?.classList.add("hidden"), 300);
    }, 300);

    Widget.self.ready();
  }, []);

  return (
    <ConfigProvider
      componentSize="middle"
      button={{ autoInsertSpace: false }}
      theme={{
        token: {
          colorPrimary: "#175cd3",
          fontFamily: '"Segoe UI Variable", "Segoe UI", "Microsoft YaHei UI", sans-serif',
          fontSize: 14,
          colorText: enhanced ? "#101e2f" : "#192535",
          colorTextSecondary: enhanced ? "#263a50" : "#30435b",
          colorTextPlaceholder: enhanced ? "#263a50" : "#30435b",
          colorTextDisabled: "#5c6979",
          borderRadius: 10,
          colorBgContainer: "#ffffff",
          colorBgElevated: "#ffffff",
          controlHeight: 34,
        },
      }}
    >
      <SettingsDialogHost />
      <Routing />
      <ThumbnailGeneratorModal />
      <WelcomeModal />
    </ConfigProvider>
  );
}
