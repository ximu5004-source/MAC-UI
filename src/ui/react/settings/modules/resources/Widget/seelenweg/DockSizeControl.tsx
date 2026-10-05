import { useTranslation } from "react-i18next";
import { DOCK_SIZE_MIN, DOCK_SIZE_MAX, normalizeDockSize } from "libs/ui/utils/dockSize";
import cs from "./dock-size.module.css";

export function DockSizeControl({ value, onChange }: { value: number; onChange: (value: number) => void }) {
  const { t } = useTranslation();
  const size = normalizeDockSize(value);
  return <div className={cs.control}>
    <div className={cs.heading}>
      <label htmlFor="dock-size">{t("weg.overall_size")}</label>
      <output htmlFor="dock-size">{size} px</output>
    </div>
    <input id="dock-size" type="range" min={DOCK_SIZE_MIN} max={DOCK_SIZE_MAX} step={1}
      value={size} aria-valuetext={`${size} px`} aria-describedby="dock-size-hint"
      onChange={event => onChange(normalizeDockSize(event.currentTarget.valueAsNumber))} />
    <div className={cs.ends} aria-hidden="true"><span>{t("weg.size_small")}</span><span>{t("weg.size_large")}</span></div>
    <p id="dock-size-hint">{t("weg.size_hint")}</p>
  </div>;
}
