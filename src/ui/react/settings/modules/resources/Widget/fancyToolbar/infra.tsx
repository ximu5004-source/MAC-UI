import { FancyToolbarSide, HideMode } from "@macui/lib/types";
import { Icon } from "libs/ui/react/components/Icon";
import { $is_touch_primary } from "libs/ui/react/utils/signals";
import { Button, InputNumber, Select, Tooltip } from "antd";
import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { OptionsFromEnum } from "../../../shared/utils/app";
import {
  getToolbarConfig,
  setToolbarDelayToHide,
  setToolbarDelayToShow,
  setToolbarHideMode,
  setToolbarItemSize,
  setToolbarMargin,
  setToolbarPadding,
  setToolbarPosition,
} from "./application";
import { SettingsGroup, SettingsOption } from "../../../../components/SettingsBox";
import Compact from "antd/es/space/Compact";
import cs from "./preview.module.css";

export function FancyToolbarSettings() {
  const settings = getToolbarConfig();
  const isTouchPrimary = $is_touch_primary.value;
  const { t } = useTranslation();
  return (
    <>
      <SettingsGroup title={t("settings_detail.layout")} icon="TbLayoutGrid">
        <div className={cs.preview} aria-hidden="true">
          <span className={cs.caption}>{t("settings_detail.preview")}</span>
          <div className={cs.window}>
            <span />
            <span />
            <span />
          </div>
          <div
            className={cs.bar}
            data-side={settings.position}
            style={{
              "--item-size": `${settings.itemSize * .55}px`,
              "--bar-padding": `${4 + settings.padding * .3}px`,
              "--bar-margin": `${12 + settings.margin * .35}px`,
            } as CSSProperties}
          >
            <Icon iconName="TbLayoutGrid" />
            <span className={cs.line} />
            <span className={cs.spacer} />
            <Icon iconName="TbSearch" />
            <Icon iconName="TbBell" />
            <Icon iconName="TbSettings" />
          </div>
        </div>
        <SettingsOption
          label={t("toolbar.item_size")}
          action={
            <InputNumber
              aria-label={t("toolbar.item_size")}
              value={settings.itemSize}
              min={4}
              max={100}
              suffix="px"
              onChange={(value) => value !== null && setToolbarItemSize(value)}
            />
          }
        />
        <SettingsOption
          label={t("toolbar.dock_side")}
          action={
            <Compact>
              {Object.values(FancyToolbarSide).map((side) => (
                <Button
                  key={side}
                  type={side === settings.position ? "primary" : "default"}
                  aria-pressed={side === settings.position}
                  onClick={() => setToolbarPosition(side)}
                >
                  <Icon iconName={`CgToolbar${side}`} size={18} />
                  {t(`wall.position.${side.toLowerCase()}`)}
                </Button>
              ))}
            </Compact>
          }
        />
        <SettingsOption
          label={t("toolbar.padding")}
          description={t("settings_detail.padding_hint")}
          action={
            <InputNumber
              aria-label={t("toolbar.padding")}
              value={settings.padding}
              min={0}
              max={40}
              suffix="px"
              onChange={(value) => value !== null && setToolbarPadding(value)}
            />
          }
        />
        <SettingsOption
          label={t("toolbar.margin")}
          description={t("settings_detail.margin_hint")}
          action={
            <InputNumber
              aria-label={t("toolbar.margin")}
              value={settings.margin}
              min={0}
              max={40}
              suffix="px"
              onChange={(value) => value !== null && setToolbarMargin(value)}
            />
          }
        />
      </SettingsGroup>
      <SettingsGroup title={t("settings_detail.behavior")} icon="TbAdjustments">
        <SettingsOption
          wide
          label={t("toolbar.auto_hide")}
          action={
            <Tooltip title={isTouchPrimary ? t("toolbar.auto_hide_touch_disabled") : undefined}>
              <Select
                aria-label={t("toolbar.auto_hide")}
                style={{ width: 180 }}
                value={settings.hideMode}
                options={OptionsFromEnum(t, HideMode, "toolbar.hide_mode")}
                onChange={setToolbarHideMode}
                disabled={isTouchPrimary}
              />
            </Tooltip>
          }
        />
        <SettingsOption
          label={t("toolbar.delay_to_show")}
          action={
            <InputNumber
              aria-label={t("toolbar.delay_to_show")}
              value={settings.delayToShow}
              min={0}
              max={10000}
              suffix="ms"
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => value !== null && setToolbarDelayToShow(value)}
            />
          }
        />
        <SettingsOption
          label={t("toolbar.delay_to_hide")}
          action={
            <InputNumber
              aria-label={t("toolbar.delay_to_hide")}
              value={settings.delayToHide}
              min={0}
              max={10000}
              suffix="ms"
              disabled={settings.hideMode === HideMode.Never || isTouchPrimary}
              onChange={(value) => value !== null && setToolbarDelayToHide(value)}
            />
          }
        />
      </SettingsGroup>
    </>
  );
}
