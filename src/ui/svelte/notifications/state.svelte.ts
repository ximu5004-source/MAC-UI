import { Settings } from "@macui/lib";
import { communicationNotifications } from "libs/ui/svelte/runes/CommunicationNotifications.svelte";
import { locale } from "./i18n/index.ts";
import { lazyRune } from "libs/ui/svelte/utils/LazyRune.svelte.ts";

const settings = lazyRune(() => Settings.getAsync());
await Settings.onChange((s) => (settings.value = s));
await settings.init();

$effect.root(() => {
  $effect(() => {
    locale.set(settings.value.language);
  });
});

class State {
  get notifications() {
    return communicationNotifications.items;
  }
  get focusAssistMode() {
    return communicationNotifications.focusAssistMode;
  }
  get modeAvailable() {
    return communicationNotifications.modeAvailable;
  }
  get nativeUnavailable() {
    return communicationNotifications.nativeUnavailable;
  }
  get communicationUnavailable() {
    return communicationNotifications.communicationUnavailable;
  }
}

export const globalState = new State();
