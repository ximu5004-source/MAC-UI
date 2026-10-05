import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { lazyRune } from "libs/ui/svelte/utils";
import { locale } from "./i18n";

// No wallpaper, media, workspace-rotation or accent-color dependencies here.
const monitors = lazyRune(() => invoke(SeelenCommand.SystemGetMonitors));
const settings = lazyRune(() => invoke(SeelenCommand.StateGetSettings, { path: null }));
await subscribe(SeelenEvent.SystemMonitorsChanged, monitors.setByPayload);
await subscribe(SeelenEvent.StateSettingsChanged, settings.setByPayload);
await Promise.all([monitors.init(), settings.init()]);

const relativeMonitors = $derived.by(() => {
  const left = Math.min(0, ...monitors.value.map((m) => m.rect.left));
  const top = Math.min(0, ...monitors.value.map((m) => m.rect.top));
  return monitors.value.map((monitor) => ({
    ...monitor,
    rect: {
      left: monitor.rect.left - left, top: monitor.rect.top - top,
      right: monitor.rect.right - left, bottom: monitor.rect.bottom - top,
    },
  }));
});

$effect.root(() => {
  $effect(() => { void locale.set(settings.value.language); });
  $effect(() => {
    relativeMonitors;
    void invoke(SeelenCommand.SetAsDesktop).catch(console.error);
  });
});

class DesktopState {
  get relativeMonitors() { return relativeMonitors; }
}
export const desktopState = new DesktopState();
