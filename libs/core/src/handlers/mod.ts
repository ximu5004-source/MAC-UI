import type { MacUiCommandArgument, MacUiCommandReturn, MacUiEventPayload } from "@macui/types";
import { invoke as tauriInvoke, type InvokeOptions } from "@tauri-apps/api/core";
import { type EventCallback, listen, type Options as ListenerOptions } from "@tauri-apps/api/event";

import type { MacUiCommand } from "./commands.ts";
import type { MacUiEvent } from "./events.ts";

type $keyof<Type> = [Type] extends [never] ? keyof Type
  : Type extends Type ? keyof Type
  : never;

type UnionToIntersection<Type> = {
  [Key in $keyof<Type>]: Extract<
    Type,
    {
      [key in Key]?: unknown;
    }
  >[Key];
};

type MapNullToVoid<Obj> = {
  [K in keyof Obj]: [Obj[K]] extends [null] ? void : Obj[K];
};

type MapNullToUndefined<Obj> = {
  [K in keyof Obj]: [Obj[K]] extends [null] ? undefined : Obj[K];
};

export type AllMacUiCommandArguments = MapNullToUndefined<
  UnionToIntersection<MacUiCommandArgument>
>;
export type AllMacUiCommandReturns = MapNullToVoid<
  UnionToIntersection<MacUiCommandReturn>
>;

export type AllMacUiEventPayloads = UnionToIntersection<MacUiEventPayload>;

/**
 * Will call to the background process
 * @args Command to be called
 * @args Command arguments
 * @return Result of the command
 */
export function invoke<T extends MacUiCommand>(
  ...args: [AllMacUiCommandArguments[T]] extends [undefined] ? [
      command: T,
      args?: undefined,
      options?: InvokeOptions,
    ]
    : [
      command: T,
      args: AllMacUiCommandArguments[T],
      options?: InvokeOptions,
    ]
): Promise<AllMacUiCommandReturns[T]> {
  const [command, commandArgs, options] = args;
  return tauriInvoke(command, commandArgs, options);
}

export type UnSubscriber = () => void;

export function subscribe<T extends MacUiEvent>(
  event: T,
  cb: EventCallback<AllMacUiEventPayloads[T]>,
  options?: ListenerOptions,
): Promise<UnSubscriber> {
  return listen(event, cb, options);
}

export * from "./events.ts";
export * from "./commands.ts";
