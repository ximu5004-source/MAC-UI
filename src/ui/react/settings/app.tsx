import { useDarkMode } from "libs/ui/react/utils/styling.ts";
import { ConfigProvider, theme } from "antd";
import { useEffect } from "react";
import { Widget } from "@seelen-ui/lib";

import { Routing } from "./router.tsx";
import { ThumbnailGeneratorModal } from "./components/ThumbnailGeneratorModal/index.tsx";
import { WelcomeModal } from "./components/WelcomeModal/infra.tsx";

export function App() {
  const isDarkMode = useDarkMode();

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
      theme={{
        token: {
          colorPrimary: isDarkMode ? "#479bff" : "#0969da",
          fontFamily: '"Segoe UI Variable", "Segoe UI", "Microsoft YaHei UI", sans-serif',
          fontSize: 13,
          borderRadius: 9,
          colorBgContainer: isDarkMode ? "#30343d" : "#ffffff",
          colorBgElevated: isDarkMode ? "#30343d" : "#ffffff",
          controlHeight: 32,
        },
        algorithm: isDarkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
      }}
    >
      <Routing />
      <ThumbnailGeneratorModal />
      <WelcomeModal />
    </ConfigProvider>
  );
}
