export class Rect {
  left = 0;
  top = 0;
  right = 0;
  bottom = 0;
}

export function isMacUiRuntime(): boolean {
  // deno-lint-ignore no-explicit-any
  return !!(globalThis.window as any).__MACUI_WIDGET;
}

export { RuntimeStyleSheet } from "./DOM.ts";
