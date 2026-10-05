import { useTranslation } from "react-i18next";

import cs from "./ColorPalette.module.css";

const COLOR_NAMES = [
  "gray",
  "red",
  "orange",
  "yellow",
  "green",
  "seafoam",
  "cyan",
  "blue",
  "indigo",
  "purple",
  "fuchsia",
  "magenta",
] as const;

const SHADES = [25, 50, 75, 100, 200, 300, 400, 500, 600, 700, 800, 900] as const;

export function ColorPalette() {
  const { t } = useTranslation();

  return (
    <div className={cs.palette}>
      <p className={cs.note}>{t("devtools.color_palette_note")}</p>
      {COLOR_NAMES.map((name) => (
        <div key={name} className={cs.colorRow}>
          <span className={cs.colorName}>{name}</span>
          <div className={cs.swatches}>
            {SHADES.map((shade) => (
              <div key={shade} className={cs.swatch}>
                <div
                  className={cs.swatchColor}
                  style={{ backgroundColor: `var(--color-${name}-${shade})` }}
                />
                <span className={cs.swatchLabel}>{shade}</span>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
