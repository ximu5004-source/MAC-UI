import { invoke, MacUiCommand } from "@macui/lib";
import { settings } from "../../../../state/mod";
import type { MacUiWegSettings } from "@macui/lib/types";
import { normalizeDockSize } from "libs/ui/utils/dockSize";

/**
 * Patches the MacUiWeg configuration with partial updates.
 * This helper function simplifies updating the weg settings by handling
 * the nested structure automatically.
 *
 * @example
 * patchWegConfig({ enabled: true, margin: 10 });
 */
export function patchWegConfig(patch: Partial<MacUiWegSettings>) {
  settings.value = {
    ...settings.value,
    byWidget: {
      ...settings.value.byWidget,
      "@macui/weg": {
        ...settings.value.byWidget["@macui/weg"],
        ...patch,
        ...(patch.size !== undefined ? { size: normalizeDockSize(patch.size) } : {}),
      },
    },
  };
}

/**
 * Gets the current MacUiWeg configuration
 */
export function getWegConfig(): MacUiWegSettings {
  return settings.value.byWidget["@macui/weg"];
}

/**
 * Imports pinned items from Windows taskbar
 */
export async function importFromWindowsTaskbar(): Promise<number> {
  return await invoke(MacUiCommand.WegImportPinnedTaskbarItems);
}
