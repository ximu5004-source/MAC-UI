import { dialog } from "@macui/lib/tauri";
import { path } from "@tauri-apps/api";
import { needRestart, settings } from "../../state/mod";
import { invoke, MacUiCommand } from "@macui/lib";

export async function simulatePerm(widgetId: string, perm: string): Promise<void> {
  await invoke(MacUiCommand.SimulatePerm, { widgetId, perm });
}

/**
 * Gets the devTools setting
 */
export function getDevTools(): boolean {
  return settings.value.devTools;
}

/**
 * Sets the devTools setting
 */
export function setDevTools(devTools: boolean) {
  settings.value = {
    ...settings.value,
    devTools,
  };
}

export function getUnlockShortcuts(): boolean {
  return settings.value.unlockShortcuts;
}

export function setUnlockShortcuts(unlockShortcuts: boolean) {
  settings.value = {
    ...settings.value,
    unlockShortcuts,
  };
}

export function getUnstableOptimizations(): boolean {
  return settings.value.unstableOptimizations;
}

export function setUnstableOptimizations(unstableOptimizations: boolean) {
  needRestart.value = true;
  settings.value = {
    ...settings.value,
    unstableOptimizations,
  };
}

/**
 * Loads a custom configuration file
 */
export async function LoadCustomConfigFile() {
  const file = await dialog.open({
    defaultPath: await path.homeDir(),
    multiple: false,
    title: "Select settings file",
    filters: [{ name: "settings", extensions: ["json"] }],
  });

  if (!file) {
    return;
  }

  settings.value = await invoke(MacUiCommand.StateGetSettings, { path: file });
}
