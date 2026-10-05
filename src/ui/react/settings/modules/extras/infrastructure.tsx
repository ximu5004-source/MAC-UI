import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { process } from "@seelen-ui/lib/tauri";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";

import { EnvConfig } from "../shared/config/infra.ts";
import cs from "./infra.module.css";

import {
  $backupStatus,
  getBackupSyncEnabled,
  getDrpc,
  getStreamingMode,
  setBackupSyncEnabled,
  setDrpc,
  setStreamingMode,
} from "./application.ts";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../components/SettingsBox/index.tsx";
import { SessionView } from "./session/infra.tsx";
import { session } from "../../state/session.ts";

const [isDevMode, isMsixBuild, isFixed] = await Promise.all([
  invoke(SeelenCommand.IsDevMode),
  invoke(SeelenCommand.IsAppxPackage),
  invoke(SeelenCommand.HasFixedRuntime),
]);

export function Information() {
  const drpc = getDrpc();
  const streamingMode = getStreamingMode();
  const backupSyncEnabled = getBackupSyncEnabled();
  const backupStatus = $backupStatus.value;

  const { t } = useTranslation();

  function onToggleDrpc(value: boolean) {
    setDrpc(value);
  }

  function onToggleStreamingMode(value: boolean) {
    setStreamingMode(value);
  }

  function onToggleBackupSync(value: boolean) {
    setBackupSyncEnabled(value);
  }

  const lastSyncLabel = backupStatus.lastSync
    ? new Date(backupStatus.lastSync).toLocaleString()
    : t("extras.backup_never_synced");

  return (
    <div className={cs.info}>
      <SettingsGroup>
        <SettingsSubGroup label="MAC UI">
          <SettingsOption label={t("mac_ui.publisher")} action={<strong>JONA</strong>} />
          <SettingsOption
            label={t("mac_ui.license")}
            description={t("mac_ui.attribution")}
            action={
              <a href="https://www.gnu.org/licenses/agpl-3.0.html" target="_blank" rel="noreferrer">
                AGPL-3.0-or-later
              </a>
            }
          />
          <SettingsOption
            label={t("mac_ui.upstream")}
            action={<a href="https://github.com/eythaann/Seelen-UI" target="_blank" rel="noreferrer">Seelen UI</a>}
          />
          <SettingsOption
            label={t("extras.version")}
            description={isFixed ? t("extras.version_fixed") : false}
            action={
              <span className={cs.version}>
                v{EnvConfig.version} {isDevMode && "(dev)"} {isMsixBuild && "(msix)"} {isFixed && "(fixed)"}
              </span>
            }
          />
        </SettingsSubGroup>
        <SettingsOption label={t("mac_ui.updates")} description={t("mac_ui.manual_updates")} />
      </SettingsGroup>

      <SettingsGroup>
        <SettingsSubGroup label={t("mac_ui.upstream_account")}>
          <div className={cs.sessionContainer}>
            <SessionView />
          </div>
        </SettingsSubGroup>
      </SettingsGroup>

      {session.value && (
        <SettingsGroup>
          <SettingsSubGroup
            label={
              <SettingsOption
                label={t("extras.backup_group")}
                action={<span className={cs.version}>{lastSyncLabel}</span>}
              />
            }
          >
            <SettingsOption
              label={t("extras.backup_sync")}
              description={t("extras.backup_sync_description")}
              action={<Switch value={backupSyncEnabled} onChange={onToggleBackupSync} />}
            />
          </SettingsSubGroup>
        </SettingsGroup>
      )}

      <SettingsGroup>
        <SettingsOption
          label={t("extras.discord_rpc")}
          description={t("extras.discord_rpc_description")}
          action={<Switch value={drpc} onChange={onToggleDrpc} />}
        />

        <SettingsOption
          label={t("extras.streaming_mode")}
          description={t("extras.streaming_mode_description")}
          action={<Switch value={streamingMode} onChange={onToggleStreamingMode} />}
        />
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption>
          <b style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            {t("extras.clear_icons")}
            <Tooltip title={t("extras.clear_icons_tooltip")}>
              <Icon iconName="LuCircleHelp" />
            </Tooltip>
          </b>
          <Button
            type="dashed"
            danger
            onClick={() => invoke(SeelenCommand.StateDeleteCachedIcons)}
            style={{ width: "50px" }}
          >
            <Icon iconName="IoReload" size={12} />
          </Button>
        </SettingsOption>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption>
          <b>{t("extras.relaunch")}</b>
          <Button type="dashed" onClick={() => process.relaunch()} style={{ width: "50px" }}>
            <Icon iconName="IoReload" size={12} />
          </Button>
        </SettingsOption>
        <SettingsOption>
          <b>{t("extras.exit")}</b>
          <Button type="dashed" danger onClick={() => process.exit(0)} style={{ width: "50px" }}>
            <Icon iconName="IoClose" />
          </Button>
        </SettingsOption>
      </SettingsGroup>
    </div>
  );
}
