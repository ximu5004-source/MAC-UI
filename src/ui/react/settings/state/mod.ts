import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { batch, computed, effect, signal } from "@preact/signals";
import { Modal } from "antd";
import { monitors } from "./system";
import { cloneDeep } from "lodash";
import i18n from "../i18n";
import { lazySignal } from "libs/ui/react/utils/LazySignal";

export const DEFAULT_SETTINGS = await invoke(SeelenCommand.StateGetDefaultSettings);

export const settings = lazySignal(() => invoke(SeelenCommand.StateGetSettings, { path: null }));
const initialSettings = signal("");
// Subscribe before fetching: a disk refresh during startup must not leave this
// window displaying an older disabled Launchpad snapshot.
await subscribe(SeelenEvent.StateSettingsChanged, ({ payload }) => {
  batch(() => {
    settings.value = payload;
    initialSettings.value = JSON.stringify(payload);
  });
});
await settings.init();
initialSettings.value = JSON.stringify(settings.value);

export const language = computed(() => settings.value.language);

export const hasChanges = computed(() => initialSettings.value !== JSON.stringify(settings.value));
export const needRestart = signal(false);

const bundledAppConfigs = await invoke(SeelenCommand.StateGetSettingsByApp);
export const appsConfig = computed(() => [...bundledAppConfigs, ...settings.value.byApp]);

export async function saveSettings() {
  const s = settings.value;

  const referenced = new Set<string>();
  const wallConfig = s.byWidget["@seelen/wallpaper-manager"];
  if (wallConfig?.defaultCollection) referenced.add(wallConfig.defaultCollection);
  Object.values(s.monitorsV3).forEach((m) => {
    if (m.wallpaperCollection) referenced.add(m.wallpaperCollection);
    Object.values(m.byWorkspace ?? {}).forEach((ws) => {
      if (ws.wallpaperCollection) referenced.add(ws.wallpaperCollection);
    });
  });

  const cleaned = {
    ...s,
    wallpaperCollections: s.wallpaperCollections.filter((c) => !c.hidden || referenced.has(c.id)),
  };

  try {
    await invoke(SeelenCommand.StateWriteSettings, {
      settings: cleaned,
    });
    initialSettings.value = JSON.stringify(cleaned);
    settings.value = cleaned;
    return true;
  } catch (error) {
    Modal.error({
      title: i18n.t("mac_ui.save_failed"),
      content: String(error),
      centered: true,
    });
    return false;
  }
}

const defaultMonitorConfig = await invoke(SeelenCommand.StateGetDefaultMonitorSettings);
effect(() => {
  const sanitized = settings.peek();
  for (const monitor of monitors.value) {
    if (!sanitized.monitorsV3[monitor.id]) {
      sanitized.monitorsV3[monitor.id] = cloneDeep(defaultMonitorConfig);
    }
  }
  batch(() => {
    settings.value = sanitized;
    initialSettings.value = JSON.stringify(sanitized);
  });
});

effect(() => {
  i18n.changeLanguage(language.value);
});

export function restoreToLastSaved() {
  settings.value = JSON.parse(initialSettings.value);
}
