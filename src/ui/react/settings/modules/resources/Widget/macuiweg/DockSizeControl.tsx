import { useTranslation } from "react-i18next";
import type { CSSProperties } from "react";
import { Icon } from "libs/ui/react/components/Icon";
import { DOCK_SIZE_MAX, DOCK_SIZE_MIN, normalizeDockSize } from "libs/ui/utils/dockSize";
import cs from "./dock-size.module.css";

export function DockSizeControl({ value, onChange, position = "Bottom" }: {
  value: number;
  onChange: (value: number) => void;
  position?: string;
}) {
  const { t } = useTranslation();
  const size = normalizeDockSize(value);
  return (
    <div className={cs.control} data-setting>
      <div className={cs.preview} aria-hidden="true">
        <span className={cs.previewLabel}>{t("settings_detail.preview")}</span>
        <div className={cs.previewWindow}>
          <i />
          <i />
          <i />
          <span />
        </div>
        <div
          className={cs.dock}
          data-dock-preview
          data-side={position}
          style={{ "--preview-size": `${size * .33}px` } as CSSProperties}
        >
          {(["RiFolder3Fill", "RiCompass3Fill", "RiImageFill", "RiApps2Fill", "RiSettings3Fill"] as const).map((
            icon,
            index,
          ) => (
            <span key={icon} className={cs.appIcon} data-color={index}>
              <Icon iconName={icon} />
            </span>
          ))}
        </div>
      </div>
      <div className={cs.adjustment}>
        <div className={cs.heading}>
          <label htmlFor="dock-size">{t("weg.overall_size")}</label>
          <output htmlFor="dock-size">
            {size}
            <small>px</small>
          </output>
        </div>
        <input
          id="dock-size"
          type="range"
          min={DOCK_SIZE_MIN}
          max={DOCK_SIZE_MAX}
          step={1}
          value={size}
          aria-valuetext={`${size} px`}
          aria-describedby="dock-size-hint"
          onChange={(event) => onChange(normalizeDockSize(event.currentTarget.valueAsNumber))}
        />
        <div className={cs.ends} aria-hidden="true">
          <span>{t("weg.size_small")}</span>
          <span>{t("weg.size_large")}</span>
        </div>
        <p id="dock-size-hint">{t("settings_detail.size_hint")}</p>
      </div>
    </div>
  );
}
