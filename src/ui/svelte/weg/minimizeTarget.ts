import type { Rect } from "@macui/lib/types";

/** Preview triggers are IPC input, so never trust a cast of customArgs. */
export function readMinimizeTarget(value: unknown): Rect | null {
  if (!value || typeof value !== "object") return null;
  const rect = value as Rect;
  if (![rect.left, rect.top, rect.right, rect.bottom].every((v) => Number.isSafeInteger(v))
    || rect.right <= rect.left || rect.bottom <= rect.top
    || rect.right - rect.left > 512 || rect.bottom - rect.top > 512) return null;
  return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
}

/** Convert CSS pixels using the actual webview origin, not its smaller hitbox. */
export function minimizeTarget(
  icon: { left: number; top: number; width: number; height: number },
  origin: { x: number; y: number },
  scale: number,
): Rect | null {
  if (![icon.left, icon.top, icon.width, icon.height, origin.x, origin.y, scale].every(Number.isFinite)
    || scale <= 0 || icon.width <= 0 || icon.height <= 0) return null;
  return {
    left: origin.x + Math.round(icon.left * scale),
    top: origin.y + Math.round(icon.top * scale),
    right: origin.x + Math.round((icon.left + icon.width) * scale),
    bottom: origin.y + Math.round((icon.top + icon.height) * scale),
  };
}
