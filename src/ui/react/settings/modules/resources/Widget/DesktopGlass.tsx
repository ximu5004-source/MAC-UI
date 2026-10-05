import { Button } from "antd";
import { useTranslation } from "react-i18next";
import type { CSSProperties } from "react";
import { SettingsGroup } from "../../../components/SettingsBox";
import { Icon } from "libs/ui/react/components/Icon";
import { DEFAULT_GLASS_APPEARANCE, GLASS_SETTINGS_WIDGET, glassAppearanceVariables, normalizeGlassAppearance, type GlassAppearance } from "libs/ui/utils/glassAppearance";
import { getWidgetConfig, patchWidgetConfig } from "./application";
import { DockSizeControl } from "./seelenweg/DockSizeControl";
import { getWegConfig, patchWegConfig } from "./seelenweg/application";
import cs from "./desktop-glass.module.css";
import "libs/ui/styles/glass-material.css";

export function GlassAppearanceControls({ value, onChange }: {
  value: GlassAppearance;
  onChange: (value: GlassAppearance) => void;
}) {
  const { t } = useTranslation();
  const range = (key: keyof GlassAppearance) => (
    <div className={cs.control} key={key}>
      <div className={cs.controlLabel}>
        <label htmlFor={`glass-${key}`}>{t(`desktop_glass.${key}`)}</label>
        <output htmlFor={`glass-${key}`}>{value[key]}%</output>
      </div>
      <input id={`glass-${key}`} type="range" min={0} max={100} step={1}
        value={value[key]} aria-valuetext={`${value[key]}%`}
        onChange={event => onChange({ ...value, [key]: event.currentTarget.valueAsNumber })} />
      <div className={cs.rangeEnds} aria-hidden="true">
        <span>{t(key.includes("Transparency") ? "desktop_glass.opaque" : "desktop_glass.matte")}</span>
        <span>{t(key.includes("Transparency") ? "desktop_glass.transparent" : "desktop_glass.glossy")}</span>
      </div>
    </div>
  );
  return <section className={cs.section} aria-labelledby="desktop-glass-heading">
    <div className={cs.heading}>
      <div>
        <h2 id="desktop-glass-heading">{t("desktop_glass.title")}</h2>
        <p>{t("desktop_glass.save_hint")}</p>
      </div>
      <Button onClick={() => onChange({ ...DEFAULT_GLASS_APPEARANCE })}>{t("desktop_glass.reset")}</Button>
    </div>
    <div className={cs.preview} style={glassAppearanceVariables(value) as CSSProperties} role="img" aria-label={t("desktop_glass.preview_description")}>
      <span className={cs.previewLabel}>{t("desktop_glass.preview")}</span>
      <div className={`mac-dock-material ${cs.previewDock}`} data-mac-material="dock">
        <div className="bg-layers"><div className="bg-layer-1" /><div className="bg-layer-2" /><div className="bg-layer-3" /></div>
        <div className={cs.previewIcons}>
          {(["RiApps2Fill", "RiFolder3Fill", "RiCompass3Fill", "RiImageFill", "RiSettings3Fill"] as const).map((icon, index) =>
            <div key={icon} className={cs.previewIcon}>
              <div className={cs.previewArtwork} data-icon-index={index}><Icon iconName={icon} size={32} /></div>
              <span className={cs.previewCoat} />
            </div>)}
        </div>
      </div>
    </div>
    <SettingsGroup>
      <div className={cs.groupHeading}><h3>{t("desktop_glass.dock")}</h3><p>{t("desktop_glass.dock_hint")}</p></div>
      {range("dockTransparency")}{range("dockGloss")}
    </SettingsGroup>
  </section>;
}

export function DesktopGlassSettings() {
  const config = getWidgetConfig(GLASS_SETTINGS_WIDGET);
  return <>
    <SettingsGroup><DockSizeControl value={getWegConfig().size} onChange={size => patchWegConfig({ size })} /></SettingsGroup>
    <GlassAppearanceControls value={normalizeGlassAppearance(config?.glassAppearance)}
      onChange={value => patchWidgetConfig(GLASS_SETTINGS_WIDGET, { glassAppearance: normalizeGlassAppearance(value) })} />
  </>;
}
