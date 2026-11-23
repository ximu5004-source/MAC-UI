import { computed, effect } from "@preact/signals";
import type { WidgetId } from "@macui/lib/types";
import { settings } from "./mod";

const SETTINGS_WIDGET = "@macui/settings" as WidgetId;
export type ColorScheme = "light" | "dark";

// MAC UI has its own appearance preference. The first launch is light even
// when Windows uses dark mode; this does not change Windows' system setting.
export const colorScheme = computed<ColorScheme>(() =>
  settings.value.byWidget[SETTINGS_WIDGET]?.colorScheme === "dark" ? "dark" : "light"
);

effect(() => {
  const mode = colorScheme.value;
  document.documentElement.dataset.glassContrast = mode === "dark" ? "enhanced" : "original";
});

export function setColorScheme(mode: ColorScheme) {
  settings.value = {
    ...settings.value,
    byWidget: {
      ...settings.value.byWidget,
      [SETTINGS_WIDGET]: {
        enabled: true,
        ...settings.value.byWidget[SETTINGS_WIDGET],
        colorScheme: mode,
      },
    },
  };
}
