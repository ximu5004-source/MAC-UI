import { settings } from "../../../../../state/mod";
import type { Border } from "@macui/lib/types";

/**
 * Patches the WindowManager Border configuration with partial updates.
 *
 * @example
 * patchBorderConfig({ enabled: true, width: 2 });
 */
export function patchBorderConfig(patch: Partial<Border>) {
  const currentWmSettings = settings.value.byWidget["@macui/window-manager"];

  settings.value = {
    ...settings.value,
    byWidget: {
      ...settings.value.byWidget,
      "@macui/window-manager": {
        ...currentWmSettings,
        border: {
          ...currentWmSettings.border,
          ...patch,
        },
      },
    },
  };
}

/**
 * Gets the current Border configuration
 */
export function getBorderConfig(): Border {
  return settings.value.byWidget["@macui/window-manager"].border;
}

/**
 * Sets the border enabled state
 */
export function setBorderEnabled(enabled: boolean) {
  patchBorderConfig({ enabled });
}

/**
 * Sets the border offset
 */
export function setBorderOffset(offset: number) {
  patchBorderConfig({ offset });
}

/**
 * Sets the border width
 */
export function setBorderWidth(width: number) {
  patchBorderConfig({ width });
}
