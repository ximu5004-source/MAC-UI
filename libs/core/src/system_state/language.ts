import { MacUiCommand, MacUiEvent, type UnSubscriber } from "../handlers/mod.ts";
import { List } from "../utils/List.ts";
import { newFromInvoke, newOnEvent } from "../utils/State.ts";
import type { SystemLanguage } from "@macui/types";

export class LanguageList extends List<SystemLanguage> {
  static getAsync(): Promise<LanguageList> {
    return newFromInvoke(this, MacUiCommand.SystemGetLanguages);
  }

  static onChange(cb: (payload: LanguageList) => void): Promise<UnSubscriber> {
    return newOnEvent(cb, this, MacUiEvent.SystemLanguagesChanged);
  }
}
