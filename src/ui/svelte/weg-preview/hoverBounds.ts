import type { Rect } from "@macui/lib/types";

export function readHoverAnchor(value: unknown): Rect | null {
  if (!value || typeof value !== "object") return null;
  const rect = value as Rect;
  return [rect.left, rect.top, rect.right, rect.bottom].every(Number.isSafeInteger) &&
      rect.right > rect.left && rect.bottom > rect.top ? rect : null;
}

export function containsPoint(rect: Rect, x: number, y: number): boolean {
  return x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom;
}
