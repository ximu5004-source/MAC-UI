export interface StackPoint {
  x: number;
  y: number;
}
export interface StackSize {
  width: number;
  height: number;
}
export type StackRect = StackPoint & StackSize;
export type ResizeEdge = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
export const STACK_GRID = 20;
export const MIN_STACK_SIZE: StackSize = { width: 160, height: 200 };

export function alignmentGridVisible(showGrid: boolean, interacting: boolean, snap: boolean): boolean {
  return showGrid || (interacting && snap);
}

export function snapToGrid(value: number): number {
  return Math.round(value / STACK_GRID) * STACK_GRID;
}

export function moveStackPosition(point: StackPoint, bounds: StackSize, size: StackSize, snap: boolean): StackPoint {
  return clampStackPosition(snap ? { x: snapToGrid(point.x), y: snapToGrid(point.y) } : point, bounds, size);
}

export function clampStackSize(size: StackSize, bounds: StackSize): StackSize {
  return {
    width: Math.round(Math.min(bounds.width, Math.max(MIN_STACK_SIZE.width, Number.isFinite(size.width) ? size.width : 260))),
    height: Math.round(Math.min(bounds.height, Math.max(MIN_STACK_SIZE.height, Number.isFinite(size.height) ? size.height : 320))),
  };
}

/** Resize moving edges on the same absolute grid as dragging; opposite edges stay anchored. */
export function resizeStackRect(rect: StackRect, delta: StackPoint, edge: ResizeEdge, bounds: StackSize, snap: boolean): StackRect {
  const align = snap ? snapToGrid : Math.round;
  const minWidth = Math.min(MIN_STACK_SIZE.width, bounds.width);
  const minHeight = Math.min(MIN_STACK_SIZE.height, bounds.height);
  let left = rect.x, top = rect.y, right = rect.x + rect.width, bottom = rect.y + rect.height;
  if (edge.includes("w")) left = Math.max(0, Math.min(right - minWidth, align(left + delta.x)));
  if (edge.includes("e")) right = Math.min(bounds.width, Math.max(left + minWidth, align(right + delta.x)));
  if (edge.includes("n")) top = Math.max(0, Math.min(bottom - minHeight, align(top + delta.y)));
  if (edge.includes("s")) bottom = Math.min(bounds.height, Math.max(top + minHeight, align(bottom + delta.y)));
  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function isStackSize(value: unknown): value is StackSize {
  if (!value || typeof value !== "object") return false;
  const size = value as StackSize;
  return Number.isFinite(size.width) && size.width > 0 && Number.isFinite(size.height) && size.height > 0;
}

export function clampStackPosition(point: StackPoint, bounds: StackSize, size: StackSize): StackPoint {
  return {
    x: Math.round(
      Math.max(0, Math.min(Number.isFinite(point.x) ? point.x : 0, Math.max(0, bounds.width - size.width))),
    ),
    y: Math.round(
      Math.max(0, Math.min(Number.isFinite(point.y) ? point.y : 0, Math.max(0, bounds.height - size.height))),
    ),
  };
}

export function defaultStackPosition(index: number, bounds: StackSize): StackPoint {
  const width = Math.min(260, bounds.width);
  const columns = Math.max(1, Math.floor((bounds.width + 14) / (width + 14)));
  return {
    x: Math.max(0, bounds.width - width - (index % columns) * (width + 14)),
    y: Math.floor(index / columns) * Math.min(420, bounds.height / 2),
  };
}

export function isStackPoint(value: unknown): value is StackPoint {
  if (!value || typeof value !== "object") return false;
  const point = value as StackPoint;
  return Number.isFinite(point.x) && Number.isFinite(point.y);
}
