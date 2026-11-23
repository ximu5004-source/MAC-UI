import { lazySignal } from "libs/ui/react/utils/LazySignal";
import { invoke, MacUiCommand, MacUiEvent, subscribe } from "@macui/lib";
import type { ResourceText } from "@macui/lib/types";
import { computed } from "@preact/signals";
import { getResourceText } from "@shared";
import { language } from "./mod";

function SorterByDisplayName(
  a: { metadata: { displayName: ResourceText } },
  b: { metadata: { displayName: ResourceText } },
): number {
  const aDisplayName = getResourceText(a.metadata.displayName, language.value);
  const bDisplayName = getResourceText(b.metadata.displayName, language.value);

  return aDisplayName.localeCompare(bDisplayName, language.value);
}

const _widgets = lazySignal(() => invoke(MacUiCommand.StateGetWidgets));
subscribe(MacUiEvent.StateWidgetsChanged, _widgets.setByPayload);
await _widgets.init();

const _plugins = lazySignal(() => invoke(MacUiCommand.StateGetPlugins));
subscribe(MacUiEvent.StatePluginsChanged, _plugins.setByPayload);
await _plugins.init();

const _wallpapers = lazySignal(() => invoke(MacUiCommand.StateGetWallpapers));
subscribe(MacUiEvent.StateWallpapersChanged, _wallpapers.setByPayload);
await _wallpapers.init();

export const widgets = computed(() => _widgets.value.toSorted(SorterByDisplayName));
export const plugins = computed(() => _plugins.value.toSorted(SorterByDisplayName));
export const wallpapers = computed(() => _wallpapers.value.toSorted(SorterByDisplayName));
