import { Switch } from "antd";
import { useTranslation } from "react-i18next";

import {
  getDevTools,
  getUnlockShortcuts,
  getUnstableOptimizations,
  setDevTools,
  setUnlockShortcuts,
  setUnstableOptimizations,
} from "./application.ts";

import { SettingsGroup, SettingsOption } from "../../components/SettingsBox/index.tsx";
import { ColorPalette } from "./ColorPalette.tsx";
import { DevToolsSettings } from "./DevToolsSettings.tsx";
import { SharedComponents } from "./SharedComponents.tsx";
import { WidgetsDebug } from "./WidgetsDebug.tsx";

export function DeveloperTools() {
  const devTools = getDevTools();
  const unlockShortcuts = getUnlockShortcuts();
  const unstableOptimizations = getUnstableOptimizations();

  const { t } = useTranslation();

  return (
    <>
      <SettingsGroup>
        <SettingsOption
          label={t("devtools.enable")}
          description={t("devtools.enable_description")}
          action={<Switch value={devTools} onChange={setDevTools} />}
        />
      </SettingsGroup>

      {devTools && (
        <SettingsGroup>
          <SettingsOption
            label={t("devtools.unstable_optimizations")}
            description={t("devtools.unstable_optimizations_description")}
            action={<Switch value={unstableOptimizations} onChange={setUnstableOptimizations} />}
          />
          <SettingsOption
            label={t("devtools.unlock_shortcuts")}
            description={t("devtools.unlock_shortcuts_description")}
            action={<Switch value={unlockShortcuts} onChange={setUnlockShortcuts} />}
          />
        </SettingsGroup>
      )}

      {devTools && (
        <>
          <h3>{t("devtools.tools")}</h3>
          <DevToolsSettings />
          <h3>{t("devtools.color_palette")}</h3>
          <ColorPalette />
          <h3>{t("devtools.shared_components")}</h3>
          <SharedComponents />
          <h3>{t("devtools.widgets_debug.tab")}</h3>
          <WidgetsDebug />
        </>
      )}
    </>
  );
}
