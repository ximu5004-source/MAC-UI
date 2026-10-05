// The persisted value is the icon cell size in logical pixels, never image zoom.
export const DOCK_SIZE_MIN = 16;
export const DOCK_SIZE_MAX = 128;
export const DOCK_SIZE_DEFAULT = 40;

export function normalizeDockSize(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.max(DOCK_SIZE_MIN, Math.min(DOCK_SIZE_MAX, Math.round(value)))
    : DOCK_SIZE_DEFAULT;
}
