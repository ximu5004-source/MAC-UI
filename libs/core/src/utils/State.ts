import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen, type Options as ListenerOptions } from "@tauri-apps/api/event";

import type {
  AllMacUiCommandArguments,
  AllMacUiCommandReturns,
  AllMacUiEventPayloads,
  MacUiCommand,
  MacUiEvent,
  UnSubscriber,
} from "../handlers/mod.ts";

// deno-lint-ignore no-explicit-any
interface ConstructorWithSingleArg<T = any> {
  // deno-lint-ignore no-explicit-any
  new (arg0: T): any;
}

export async function newFromInvoke<
  Command extends MacUiCommand,
  This extends ConstructorWithSingleArg<AllMacUiCommandReturns[Command]>,
>(
  Class: This,
  command: Command,
  args?: NonNullable<AllMacUiCommandArguments[Command]>,
): Promise<InstanceType<This>> {
  return new Class(await tauriInvoke(command, args));
}

export function newOnEvent<
  Event extends MacUiEvent,
  This extends ConstructorWithSingleArg<AllMacUiEventPayloads[Event]>,
>(
  cb: (instance: InstanceType<This>) => void,
  Class: This,
  event: Event,
  options?: ListenerOptions,
): Promise<UnSubscriber> {
  return tauriListen(
    event,
    (eventData) => {
      cb(new Class(eventData.payload as AllMacUiEventPayloads[Event]));
    },
    options,
  );
}
