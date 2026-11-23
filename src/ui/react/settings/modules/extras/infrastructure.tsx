import { invoke, MacUiCommand } from "@macui/lib";
import { process } from "@macui/lib/tauri";
import { Icon } from "libs/ui/react/components/Icon";
import { Button, Switch } from "antd";
import { useTranslation } from "react-i18next";
import { EnvConfig } from "../shared/config/infra";
import { getStreamingMode, setStreamingMode } from "./application";
import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../components/SettingsBox";
import cs from "./infra.module.css";
import { QuitButton } from "../../components/header/QuitButton";

export function Information() {
  const { t } = useTranslation();
  return (
    <div className={cs.info}>
      <SettingsGroup>
        <SettingsSubGroup label="MAC UI">
          <SettingsOption label={t("mac_ui.publisher")} action={<strong>JONA</strong>} />
          <SettingsOption label={t("extras.version")} action={<span>v{EnvConfig.version}</span>} />
          <SettingsOption label={t("mac_local.local_mode")} description={t("mac_local.local_hint")} />
          <SettingsOption label={t("mac_ui.updates")} description={t("mac_ui.manual_updates")} />
        </SettingsSubGroup>
      </SettingsGroup>
      <SettingsGroup>
        <SettingsOption
          label={t("extras.streaming_mode")}
          description={t("extras.streaming_mode_description")}
          action={<Switch checked={getStreamingMode()} onChange={setStreamingMode} />}
        />
        <SettingsOption
          label={t("extras.clear_icons")}
          description={t("extras.clear_icons_tooltip")}
          action={
            <Button
              aria-label={t("extras.clear_icons")}
              onClick={() => invoke(MacUiCommand.StateDeleteCachedIcons)}
            >
              <Icon iconName="IoReload" />
            </Button>
          }
        />
      </SettingsGroup>
      <SettingsGroup>
        <SettingsOption
          label={t("extras.relaunch")}
          action={<Button onClick={() => process.relaunch()}>{t("extras.relaunch")}</Button>}
        />
        <SettingsOption
          label={t("extras.exit")}
          action={<QuitButton />}
        />
      </SettingsGroup>
      <details className="mac-details">
        <summary>{t("mac_ui.license")}</summary>
        <p>{t("mac_ui.attribution")}</p>
        <p>{t("mac_local.license_notice")}</p>
        <p>
          <a href="https://www.gnu.org/licenses/agpl-3.0.html" target="_blank" rel="noreferrer">AGPL-3.0-or-later</a>
          {" · "}
          <a href="https://github.com/ximu5004-source/MAC-UI" target="_blank" rel="noreferrer">
            {t("mac_local.source")}
          </a>
        </p>
      </details>
    </div>
  );
}
