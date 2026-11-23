import type { Settings, WidgetId } from "@macui/types";
import { invoke, MacUiCommand, MacUiEvent, subscribe } from "../../handlers/mod.ts";

let initialization: Promise<void> | undefined;

/** Liquid Glass appearance is shared across every native webview, independent of Windows mode. */
export function applyColorScheme(settings: Pick<Settings, "byWidget">): void {
  const scheme = settings.byWidget["@macui/settings" as WidgetId]?.colorScheme === "dark" ? "dark" : "light";
  const root = document.documentElement;
  root.dataset.glassContrast = scheme === "dark" ? "enhanced" : "original";
  root.dataset.colorScheme = "light";
  root.style.colorScheme = "light";
  globalThis.dispatchEvent(new Event("macui-color-scheme-change"));
}

export function initializeColorScheme(): Promise<void> {
  return initialization ??= (async () => {
    // Subscribe first, so a save arriving during the initial fetch cannot be overwritten.
    let changed = false;
    await subscribe(MacUiEvent.StateSettingsChanged, ({ payload }) => {
      changed = true;
      applyColorScheme(payload);
    });
    const settings = await invoke(MacUiCommand.StateGetSettings, { path: null });
    if (!changed) applyColorScheme(settings);
  })().catch((error) => {
    document.documentElement.dataset.glassContrast = "original";
    document.documentElement.dataset.colorScheme = "light";
    document.documentElement.style.colorScheme = "light";
    console.error("MAC UI appearance initialization failed", error);
  });
}
