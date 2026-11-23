import { MacUiCommand, MacUiEvent, type UnSubscriber } from "../handlers/mod.ts";
import { newFromInvoke, newOnEvent } from "../utils/State.ts";
import type { User } from "@macui/types";

export class UserDetails {
  constructor(public user: User) {}

  static getAsync(): Promise<UserDetails> {
    return newFromInvoke(this, MacUiCommand.GetUser);
  }

  static onChange(cb: (payload: UserDetails) => void): Promise<UnSubscriber> {
    return newOnEvent(cb, this, MacUiEvent.UserChanged);
  }
}
