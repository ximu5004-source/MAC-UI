import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "libs/ui/react/components/Icon";
import { type ColorScheme, colorScheme, setColorScheme } from "../../state/colorScheme";
import { hasChanges, saveSettings } from "../../state/mod";
import cs from "./index.module.css";

export function GlassMode() {
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  async function selectGlass(mode: ColorScheme) {
    if (saving || mode === colorScheme.value) return;
    const draft = hasChanges.value;
    setColorScheme(mode);
    if (!draft) {
      setSaving(true);
      try {
        await saveSettings();
      } finally {
        setSaving(false);
      }
    }
  }
  return (
    <div className={cs.modes} role="group" aria-label={t("mac_local.color_scheme")}>
      {(["light", "dark"] as const).map((mode) => (
        <button
          type="button"
          key={mode}
          aria-pressed={colorScheme.value === mode}
          disabled={saving}
          onClick={() => void selectGlass(mode)}
        >
          <Icon iconName={mode === "light" ? "TbSun" : "TbContrast"} size={20} />
          {t(mode === "light" ? "mac_local.original_glass" : "mac_local.contrast_glass")}
        </button>
      ))}
    </div>
  );
}
