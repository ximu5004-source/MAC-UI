import { MacUiCommand, MacUiEvent, type UnSubscriber } from "../../handlers/mod.ts";
import { List } from "../../utils/List.ts";
import { newFromInvoke, newOnEvent } from "../../utils/State.ts";
import type { Plugin } from "@macui/types";
import { Widget } from "../widget/mod.ts";

export class PluginList extends List<Plugin> {
  static getAsync(): Promise<PluginList> {
    return newFromInvoke(this, MacUiCommand.StateGetPlugins);
  }

  static onChange(cb: (payload: PluginList) => void): Promise<UnSubscriber> {
    return newOnEvent(cb, this, MacUiEvent.StatePluginsChanged);
  }

  forCurrentWidget(): Plugin[] {
    const target = Widget.self.id;
    return this.inner.filter((plugin) => plugin.target === target);
  }
}
