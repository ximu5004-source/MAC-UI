import type { Rect } from "@macui/lib/types";
import { minimizeTarget, readMinimizeTarget } from "./minimizeTarget";

/** CSS matrices may contain numerical noise, but real scale/skew/rotation is
 * never interpreted as a hide translation. */
export function isDockTranslation(matrix: {
  is2D: boolean; a: number; b: number; c: number; d: number; e: number; f: number;
}): boolean {
  const epsilon = 1e-6;
  return matrix.is2D && [matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f].every(Number.isFinite)
    && Math.abs(matrix.a - 1) <= epsilon && Math.abs(matrix.d - 1) <= epsilon
    && Math.abs(matrix.b) <= epsilon && Math.abs(matrix.c) <= epsilon;
}

/** Hide-mode translates the entire #root. Its resting icon location is stable
 * throughout that transition, including before the Dock has ever been hovered. */
export function restingDockIconTarget(
  icon: { left: number; top: number; width: number; height: number },
  origin: { x: number; y: number },
  scale: number,
  translation: { x: number; y: number },
): Rect | null {
  if (![translation.x, translation.y].every(Number.isFinite)) return null;
  return readMinimizeTarget(minimizeTarget({
    left: icon.left - translation.x,
    top: icon.top - translation.y,
    // DOMRect exposes these on its prototype; object spread loses them.
    width: icon.width,
    height: icon.height,
  }, origin, scale));
}
