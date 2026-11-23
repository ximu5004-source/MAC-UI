import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import { lazyRune } from "libs/ui/svelte/utils";
import { locale } from "./i18n/index.ts";

let settings = lazyRune(() => invoke(MacUiCommand.StateGetSettings, { path: null }));
subscribe(MacUiEvent.StateSettingsChanged, settings.setByPayload);
await settings.init();
$effect.root(() => {
  $effect(() => {
    locale.set(settings.value.language);
  });
});

const langs = lazyRune(() => invoke(MacUiCommand.SystemGetLanguages));
subscribe(MacUiEvent.SystemLanguagesChanged, langs.setByPayload);
await langs.init();

export const state = {
  get langs() {
    return langs.value;
  },
};
