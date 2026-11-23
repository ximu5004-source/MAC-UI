import { HideMode, MacUiWegMode, MacUiWegSide, WegMiddleClickAction, WegWindowAnimation } from "@macui/lib/types";
import { Icon } from "libs/ui/react/components/Icon";
import { $is_touch_primary } from "libs/ui/react/utils/signals";
import { Button, InputNumber, message, Select, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";
import { OptionsFromEnum } from "../../../shared/utils/app";
import { getWegConfig, importFromWindowsTaskbar, patchWegConfig } from "./application";
import { getDevTools } from "../../../developer/application";
import { SettingsGroup, SettingsOption } from "../../../../components/SettingsBox";
import Compact from "antd/es/space/Compact";
import { DockSizeControl } from "./DockSizeControl";

export function MacUiWegSettings() {
  const settings = getWegConfig();
  const isTouchPrimary = $is_touch_primary.value;
  const devTools = getDevTools();
  const { t } = useTranslation();
  async function importTaskbar() {
    try {
      const count = await importFromWindowsTaskbar();
      if (count === 0) message.info(t("weg.import_no_items"));
      else message.success(t("weg.import_success", { count }));
    } catch (error) {
      message.error(t("weg.import_error"));
      console.error("Failed to import taskbar items:", error);
    }
  }
  return (
    <>
      <DockSizeControl
        value={settings.size}
        position={settings.position}
        onChange={(size) => patchWegConfig({ size })}
      />
      <SettingsGroup title={t("settings_detail.layout")} icon="TbLayoutGrid">
        <SettingsOption
          wide
          label={t("weg.dock_side")}
          action={
            <Compact>
              {Object.values(MacUiWegSide).map((side) => (
                <Button
                  key={side}
                  type={side === settings.position ? "primary" : "default"}
                  aria-pressed={side === settings.position}
                  onClick={() => patchWegConfig({ position: side })}
                >
                  <Icon iconName={`CgToolbar${side}`} size={18} />
                  {t(`wall.position.${side.toLowerCase()}`)}
                </Button>
              ))}
            </Compact>
          }
        />
        <SettingsOption
          label={t("weg.width")}
          action={
            <Select
              aria-label={t("weg.width")}
              style={{ width: 180 }}
              value={settings.mode}
              options={OptionsFromEnum(t, MacUiWegMode, "weg.mode")}
              onChange={(mode) => patchWegConfig({ mode })}
            />
          }
        />
        <SettingsOption
          label={t("weg.margin")}
          description={t("settings_detail.margin_hint")}
          action={
            <InputNumber
              aria-label={t("weg.margin")}
              value={settings.margin}
              min={0}
              max={40}
              onChange={(value) => value !== null && patchWegConfig({ margin: value })}
              suffix="px"
            />
          }
        />
        <SettingsOption
          label={t("weg.padding")}
          description={t("settings_detail.padding_hint")}
          action={
            <InputNumber
              aria-label={t("weg.padding")}
              value={settings.padding}
              min={0}
              max={40}
              onChange={(value) => value !== null && patchWegConfig({ padding: value })}
              suffix="px"
            />
          }
        />
        <SettingsOption
          label={t("weg.items.gap")}
          action={
            <InputNumber
              aria-label={t("weg.items.gap")}
              value={settings.spaceBetweenItems}
              min={0}
              max={40}
              onChange={(value) => value !== null && patchWegConfig({ spaceBetweenItems: value })}
              suffix="px"
            />
          }
        />
      </SettingsGroup>
      <SettingsGroup title={t("settings_detail.behavior")} icon="TbAdjustments">
        <SettingsOption
          label={t("weg.auto_hide")}
          action={
            <Tooltip title={isTouchPrimary ? t("weg.auto_hide_touch_disabled") : undefined}>
              <Select
                aria-label={t("weg.auto_hide")}
                style={{ width: 180 }}
                value={settings.hideMode}
                options={OptionsFromEnum(t, HideMode, "weg.hide_mode")}
                onChange={(hideMode) => patchWegConfig({ hideMode })}
                disabled={isTouchPrimary}
              />
            </Tooltip>
          }
        />
        <SettingsOption
          label={t("weg.window_animation.label")}
          description={t("weg.window_animation.description")}
          action={
            <Select
              aria-label={t("weg.window_animation.label")}
              style={{ width: 180 }}
              value={settings.windowAnimation}
              options={[{ value: WegWindowAnimation.Genie, label: t("weg.window_animation.genie") }, {
                value: WegWindowAnimation.Native,
                label: t("weg.window_animation.native"),
              }]}
              onChange={(windowAnimation) => patchWegConfig({ windowAnimation })}
            />
          }
        />
        <SettingsOption
          label={t("weg.delay_to_show")}
          action={
            <InputNumber
              aria-label={t("weg.delay_to_show")}
              value={settings.delayToShow}
              min={0}
              max={10000}
              suffix="ms"
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => value !== null && patchWegConfig({ delayToShow: value })}
            />
          }
        />
        <SettingsOption
          label={t("weg.delay_to_hide")}
          action={
            <InputNumber
              aria-label={t("weg.delay_to_hide")}
              value={settings.delayToHide}
              min={0}
              max={10000}
              suffix="ms"
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => value !== null && patchWegConfig({ delayToHide: value })}
            />
          }
        />
      </SettingsGroup>
      <SettingsGroup title={t("settings_detail.applications")} icon="TbApps">
        <SettingsOption
          label={t("weg.items.show_window_title")}
          action={
            <Switch
              aria-label={t("weg.items.show_window_title")}
              checked={settings.showWindowTitle}
              onChange={(showWindowTitle) => patchWegConfig({ showWindowTitle })}
            />
          }
        />
        <SettingsOption
          label={t("weg.items.show_instance_counter")}
          action={
            <Switch
              aria-label={t("weg.items.show_instance_counter")}
              checked={settings.showInstanceCounter}
              onChange={(showInstanceCounter) => patchWegConfig({ showInstanceCounter })}
            />
          }
        />
        <SettingsOption
          label={t("weg.items.split_windows")}
          action={
            <Switch
              aria-label={t("weg.items.split_windows")}
              checked={settings.splitWindows}
              onChange={(splitWindows) => patchWegConfig({ splitWindows })}
            />
          }
        />
        <SettingsOption
          label={t("weg.items.middle_click_action.label")}
          action={
            <Select
              aria-label={t("weg.items.middle_click_action.label")}
              style={{ width: 180 }}
              value={settings.middleClickAction}
              options={[{ value: WegMiddleClickAction.CloseApp, label: t("weg.items.middle_click_action.close_app") }, {
                value: WegMiddleClickAction.OpenNewInstance,
                label: t("weg.items.middle_click_action.open_new_instance"),
              }]}
              onChange={(middleClickAction) => patchWegConfig({ middleClickAction })}
            />
          }
        />
        <SettingsOption
          label={t("weg.items.temporal_visibility.label")}
          action={
            <Select
              aria-label={t("weg.items.temporal_visibility.label")}
              style={{ width: 180 }}
              value={settings.temporalItemsVisibility}
              options={[{ value: "All", label: t("weg.items.temporal_visibility.all") }, {
                value: "OnMonitor",
                label: t("weg.items.temporal_visibility.on_monitor"),
              }]}
              onChange={(temporalItemsVisibility) => patchWegConfig({ temporalItemsVisibility })}
            />
          }
        />
        <SettingsOption
          label={t("weg.items.pinned_visibility.label")}
          action={
            <Select
              aria-label={t("weg.items.pinned_visibility.label")}
              style={{ width: 180 }}
              value={settings.pinnedItemsVisibility}
              options={[{ value: "Always", label: t("weg.items.pinned_visibility.always") }, {
                value: "WhenPrimary",
                label: t("weg.items.pinned_visibility.when_primary"),
              }]}
              onChange={(pinnedItemsVisibility) => patchWegConfig({ pinnedItemsVisibility })}
            />
          }
        />
        {devTools && (
          <SettingsOption
            label={t("weg.show_end_task")}
            action={
              <Switch
                aria-label={t("weg.show_end_task")}
                checked={settings.showEndTask}
                onChange={(showEndTask) => patchWegConfig({ showEndTask })}
              />
            }
          />
        )}
      </SettingsGroup>
      <SettingsGroup title={t("weg.import_from_taskbar.label")} icon="TbDeviceDesktop">
        <SettingsOption
          label={t("settings_detail.import_hint")}
          action={<Button onClick={importTaskbar}>{t("weg.import_from_taskbar.button")}</Button>}
        />
      </SettingsGroup>
    </>
  );
}
