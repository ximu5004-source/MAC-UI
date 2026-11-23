import type { WidgetId } from "@macui/lib/types";
import { Switch } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";

import {
  getMonitorWidgetConfig,
  getWidgetConfig,
  isWidgetEnabled,
  patchWidgetConfig as patchWidget,
  patchWidgetInstanceConfig as patchInstance,
  patchWidgetMonitorConfig as patchMonitor,
  resetWidgetConfig,
} from "./application.ts";
import { widgets } from "../../../state/resources.ts";
import { getDevTools } from "../../developer/application.ts";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { getResourceText } from "libs/ui/react/utils";
import type { IconName } from "libs/ui/icons";
import cs from "../infra.module.css";
import feature from "./feature.module.css";

import { Button } from "antd";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../../components/SettingsBox/index.tsx";
import { RenderBySettingsDeclaration } from "./ConfigRenderer.tsx";
import { WidgetInstanceSelector } from "./InstanceSelector.tsx";
import { MacUiWegSettings } from "./macuiweg/infra.tsx";
import { FancyToolbarSettings } from "./fancyToolbar/infra.tsx";
import { WallSettings } from "./Wall/infra.tsx";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { WindowManagerSettings } from "./WindowManager/main/infra/index.tsx";
import { DesktopOrganizerSettings } from "./DesktopControls";

export function WidgetConfiguration({
  widgetId,
  monitorId,
  children,
}: {
  widgetId: WidgetId;
  monitorId?: string;
  children?: React.ReactNode;
}) {
  const [selectedInstance, setSelectedInstance] = useState<string | null>(null);

  const widget = widgets.value.find((t) => t.id === widgetId);
  const rootConfig = getWidgetConfig(widgetId) || {
    enabled: widget?.loader !== "Legacy" && isWidgetEnabled(widgetId),
  };

  const monitorConfig = monitorId ? getMonitorWidgetConfig(monitorId, widgetId) : undefined;
  const areDevToolsEnabled = getDevTools();

  const { t, i18n } = useTranslation();
  const enabledLabel = monitorId
    ? t("widget.enable_for_monitor")
    : widgetId === "@macui/desktop-shell"
    ? t("desktop_manager.enable")
    : widgetId === "@macui/wallpaper-manager"
    ? t("desktop_manager.enable_wallpaper")
    : t("widget.enable");

  if (!widget) {
    return <div>404</div>;
  }

  const onConfigChange = (key: string, value: any) => {
    if (monitorId) {
      patchMonitor(monitorId, widgetId, { [key]: value });
      return;
    }

    // intances `enabled` always inherit from widget root config
    if (selectedInstance && key !== "enabled") {
      patchInstance(widgetId, selectedInstance, { [key]: value });
      return;
    }

    patchWidget(widgetId, { [key]: value });
  };

  const instances = Object.keys(rootConfig.$instances || {}).map((instanceId) => ({
    label: `${t("widget.instance")} ${instanceId.slice(0, 6)}`,
    value: instanceId,
  }));

  const instanceConfig = selectedInstance ? rootConfig.$instances?.[selectedInstance] : undefined;
  const config = {
    ...rootConfig,
    ...(instanceConfig || {}),
    ...(monitorConfig || {}),
  };

  const showToggleEnabled = !monitorId || widget.instances === "ReplicaByMonitor";

  const widgetPlugins = widget.plugins;
  const name = getResourceText(widget.metadata.displayName, i18n.language);

  return (
    <>
      <div className={feature.header} data-setting data-feature-header>
        <span className={feature.icon} aria-hidden="true">
          <Icon iconName={(widget.icon || "TbLayoutGrid") as IconName} size={26} />
        </span>
        <div className={feature.identity}>
          <h2>{name}</h2>
          <p>{t(`settings_detail.features.${widgetId.split("/")[1]}`, { defaultValue: "" })}</p>
        </div>
        {showToggleEnabled && (
          <div className={feature.actions}>
            <span className={feature.status} data-enabled={!!config.enabled}>
              {t(config.enabled ? "mac_ui.enabled" : "mac_ui.disabled")}
            </span>
            <Switch
              aria-label={monitorId ? enabledLabel : t("settings_detail.toggle", { name })}
              checked={config.enabled}
              onChange={(value) => onConfigChange("enabled", value)}
            />
            <Button
              type="text"
              title={t("reset_configurations")}
              aria-label={t("reset_configurations")}
              onClick={() => resetWidgetConfig(widgetId)}
            >
              <Icon iconName="RiResetLeftLine" />
            </Button>
          </div>
        )}
      </div>
      {widgetPlugins.length > 0 && (
        <SettingsGroup>
          <SettingsSubGroup label={t("resources.included_plugins")}>
            <div className={cs.tags}>
              {widgetPlugins.map((plugin) => (
                <div key={plugin.id} className={cs.tag}>
                  <ResourceText text={plugin.metadata.displayName} />
                </div>
              ))}
            </div>
          </SettingsSubGroup>
        </SettingsGroup>
      )}

      {widget.instances === "Multiple" && (
        <SettingsGroup>
          <SettingsOption>
            <b>{t("widget.default_instance")}</b>
            <span style={{ color: "var(--color-text-3)" }}>
              {t("widget.default_instance_hint")}
            </span>
          </SettingsOption>
          <SettingsOption>
            <b>{t("widget.extra_instances")}</b>
            <WidgetInstanceSelector
              widgetId={widgetId}
              options={instances}
              selected={selectedInstance}
              onSelect={setSelectedInstance}
            />
          </SettingsOption>
        </SettingsGroup>
      )}

      <RenderBySettingsDeclaration
        definitions={widget.settings}
        values={config}
        onConfigChange={onConfigChange}
        isByMonitor={!!monitorId}
      />

      {children}

      {areDevToolsEnabled && (
        <SettingsGroup>
          <SettingsSubGroup label={<b>Raw Config</b>}>
            <pre>{JSON.stringify(rootConfig, null, 2)}</pre>
          </SettingsSubGroup>
          {!!monitorId && (
            <SettingsSubGroup label={<b>Raw Monitor Patch</b>}>
              <pre>{monitorConfig ? JSON.stringify(monitorConfig, null, 2) : "Inherited"}</pre>
            </SettingsSubGroup>
          )}
        </SettingsGroup>
      )}
    </>
  );
}

export function WidgetView() {
  const [searchParams] = useSearchParams();
  const widgetId = searchParams.get("id") as WidgetId;
  return <WidgetSettings widgetId={widgetId} />;
}

export function WidgetSettings({ widgetId }: { widgetId: WidgetId }) {
  if (widgetId === "@macui/desktop-shell") {
    return (
      <WidgetConfiguration widgetId={widgetId}>
        <DesktopOrganizerSettings />
      </WidgetConfiguration>
    );
  }
  if (widgetId === "@macui/weg") {
    return (
      <WidgetConfiguration widgetId={widgetId}>
        <MacUiWegSettings />
      </WidgetConfiguration>
    );
  }

  if (widgetId === "@macui/window-manager") {
    return (
      <WidgetConfiguration widgetId={widgetId}>
        <WindowManagerSettings />
      </WidgetConfiguration>
    );
  }

  if (widgetId === "@macui/fancy-toolbar") {
    return (
      <WidgetConfiguration widgetId={widgetId}>
        <FancyToolbarSettings />
      </WidgetConfiguration>
    );
  }

  if (widgetId === "@macui/wallpaper-manager") {
    return (
      <WidgetConfiguration widgetId={widgetId}>
        <WallSettings />
      </WidgetConfiguration>
    );
  }

  return <WidgetConfiguration widgetId={widgetId} />;
}
