import type { Rect } from "@seelen-ui/lib/types";

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
