import { Settings } from "@seelen-ui/lib";
import { lazyRune } from "../utils/LazyRune.svelte";
import { DEFAULT_GLASS_APPEARANCE, GLASS_SETTINGS_WIDGET, normalizeGlassAppearance } from "../../utils/glassAppearance";

const read = (settings: Settings) => normalizeGlassAppearance(
  settings.byWidget[GLASS_SETTINGS_WIDGET]?.glassAppearance,
);
const appearance = lazyRune(async () => read(await Settings.getAsync()));

// One listener per WebView, not one per icon. Register before fetching to avoid
// an in-flight settings fetch overwriting a more recent saved configuration.
void (async () => {
  await Settings.onChange(settings => { appearance.value = read(settings); });
  await appearance.init();
})().catch(error => console.error("Could not load glass appearance:", error));

class GlassAppearanceState {
  get value() {
    return appearance.isInitialized() ? appearance.value : DEFAULT_GLASS_APPEARANCE;
  }
}
export const glassAppearance = new GlassAppearanceState();
